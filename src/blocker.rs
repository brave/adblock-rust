use hashbrown::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::FromIterator;

#[cfg(feature = "object-pooling")]
use lifeguard::Pool;

use crate::filters::network::{NetworkFilter, NetworkMatchable};
use crate::request::Request;
use crate::utils::{fast_hash, Hash};
use crate::optimizer;
use crate::resources::{Resource, RedirectResourceStorage, RedirectResource};
use crate::utils;

pub struct BlockerOptions {
    pub enable_optimizations: bool,
}

#[derive(Debug, Serialize)]
pub struct BlockerResult {
    pub matched: bool,
    /// Normally, Brave Browser returns `200 OK` with an empty body when
    /// `matched` is `true`, except if `explicit_cancel` is also `true`, in
    /// which case the request is cancelled.
    pub explicit_cancel: bool,
    /// Important is used to signal that a rule with the `important` option
    /// matched. An `important` match means that exceptions should not apply
    /// and no further checking is neccesary--the request should be blocked
    /// (empty body or cancelled).
    ///
    /// Brave Browser keeps seperate instances of [`Blocker`] for default
    /// lists and regional ones, so `important` here is used to correct
    /// behaviour between them: checking should stop instead of moving to the
    /// next instance iff an `important` rule matched.
    pub important: bool,
    /// Iff the blocker matches a rule which has the `redirect` option, as per
    /// [uBlock Origin's redirect syntax][1], the `redirect` is `Some`. The
    /// `redirect` field contains the body of the redirect to be injected.
    ///
    /// [1]: https://github.com/gorhill/uBlock/wiki/Static-filter-syntax#redirect
    pub redirect: Option<String>,
    /// Exception is `Some` when the blocker matched on an exception rule.
    /// Effectively this means that there was a match, but the request should
    /// not be blocked. It is a non-empty string if the blocker was initialized
    /// from a list of rules with debugging enabled, otherwise the original
    /// string representation is discarded to reduce memory use.
    pub exception: Option<String>,
    /// Filter--similarly to exception--includes the string representation of
    /// the rule when there is a match and debugging is enabled. Otherwise, on
    /// a match, it is `Some`.
    pub filter: Option<String>,
    /// The `error` field is only used to signal that there was an error in
    /// parsing the provided URLs when using the simpler
    /// [`crate::engine::Engine::check_network_urls`] method.
    pub error: Option<String>,
}

impl Default for BlockerResult {
    fn default() -> BlockerResult {
        BlockerResult {
            matched: false,
            explicit_cancel: false,
            important: false,
            redirect: None,
            exception: None,
            filter: None,
            error: None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BlockerError {
    SerializationError,
    DeserializationError,
    OptimizedFilterExistence,
    BadFilterAddUnsupported,
    FilterExists,
}

#[cfg(feature = "object-pooling")]
pub struct TokenPool {
    pub pool: Pool<Vec<utils::Hash>>
}

#[cfg(feature = "object-pooling")]
impl Default for TokenPool {
    fn default() -> TokenPool {
        TokenPool {
            pool: lifeguard::pool()
                .with(lifeguard::StartingSize(1))
                .with(lifeguard::Supplier(|| Vec::with_capacity(utils::TOKENS_BUFFER_SIZE)))
                .build()
        }
    }
}

pub struct Blocker {
    pub(crate) csp: NetworkFilterList,
    pub(crate) exceptions: NetworkFilterList,
    pub(crate) importants: NetworkFilterList,
    pub(crate) redirects: NetworkFilterList,
    pub(crate) filters_tagged: NetworkFilterList,
    pub(crate) filters: NetworkFilterList,

    // Enabled tags are not serialized - when deserializing, tags of the existing
    // instance (the one we are recreating lists into) are maintained
    pub(crate) tags_enabled: HashSet<String>,
    pub(crate) tagged_filters_all: Vec<NetworkFilter>,

    // Not serialized
    pub(crate) hot_filters: NetworkFilterList,

    pub(crate) enable_optimizations: bool,

    pub(crate) resources: RedirectResourceStorage,
    // Not serialized
    #[cfg(feature = "object-pooling")]
    pub(crate) pool: TokenPool,

    pub(crate) generic_hide: NetworkFilterList,
}

impl Blocker {
    /**
     * Decide if a network request (usually from WebRequest API) should be
     * blocked, redirected or allowed.
     */
    pub fn check(&self, request: &Request) -> BlockerResult {
        self.check_parameterised(request, false, false)
    }

    pub fn check_generic_hide(&self, hostname_request: &Request) -> bool {
        let mut request_tokens;
        #[cfg(feature = "object-pooling")]
        {
            request_tokens = self.pool.pool.new();
        }
        #[cfg(not(feature = "object-pooling"))]
        {
            request_tokens = Vec::with_capacity(utils::TOKENS_BUFFER_SIZE);
        }
        hostname_request.get_tokens(&mut request_tokens);

        self.generic_hide.check(hostname_request, &request_tokens, &HashSet::new()).is_some()
    }

    pub fn check_parameterised(&self, request: &Request, matched_rule: bool, force_check_exceptions: bool) -> BlockerResult {
        if !request.is_supported {
            return BlockerResult::default();
        }

        lazy_static! {
            // only check for tags in tagged and exception rule buckets,
            // pass empty set for the rest
            static ref NO_TAGS: HashSet<String> = HashSet::new();
        }

        let mut request_tokens;
        #[cfg(feature = "object-pooling")]
        {
            request_tokens = self.pool.pool.new();
        }
        #[cfg(not(feature = "object-pooling"))]
        {
            request_tokens = Vec::with_capacity(utils::TOKENS_BUFFER_SIZE);
        }
        request.get_tokens(&mut request_tokens);

        // Check the filters in the following order:
        // 1. $important (not subject to exceptions)
        // 2. redirection ($redirect=resource)
        // 3. normal filters - if no match by then
        // 4. exceptions - if any non-important match of forced

        #[cfg(feature = "metrics")]
        print!("importants\t");
        // Always check important filters
        let important_filter = self
            .importants
            .check(request, &request_tokens, &NO_TAGS);

        // only check the rest of the rules if not previously matched
        let filter = if important_filter.is_none() && !matched_rule {
            #[cfg(feature = "metrics")]
            print!("tagged\t");
            self.filters_tagged.check(request, &request_tokens, &self.tags_enabled)
                .or_else(|| {
                    #[cfg(feature = "metrics")]
                    print!("redirects\t");
                    self.redirects.check(request, &request_tokens, &NO_TAGS)
                })
                .or_else(|| {
                    #[cfg(feature = "metrics")]
                    print!("filters\t");
                    self.filters.check(request, &request_tokens, &NO_TAGS)
                })
        } else {
            important_filter
        };

        let exception = match filter.as_ref() {
            // if no other rule matches, only check exceptions if forced to
            None if matched_rule || force_check_exceptions => {
                #[cfg(feature = "metrics")]
                print!("exceptions\t");
                self.exceptions.check(request, &request_tokens, &self.tags_enabled)
            }
            None => None,
            // If matched an important filter, exceptions don't atter
            Some(f) if f.is_important() => None,
            Some(f) if f.has_bug() => {
                #[cfg(feature = "metrics")]
                print!("exceptions\t");
                // Set `bug` of request
                let mut request_bug = request.clone();
                request_bug.bug = f.bug;
                self.exceptions.check(&request_bug, &request_tokens, &self.tags_enabled)
            }
            Some(_) => {
                #[cfg(feature = "metrics")]
                print!("exceptions\t");
                self.exceptions.check(request, &request_tokens, &self.tags_enabled)
            }
        };

        #[cfg(feature = "metrics")]
        println!();

        // only match redirects if we have them set up
        let redirect: Option<String> = filter.as_ref().and_then(|f| {
            // Filter redirect option is set
            if let Some(redirect) = f.redirect.as_ref() {
                // And we have a matching redirect resource
                if let Some(resource) = self.resources.get_resource(redirect) {
                    let data_url = format!("data:{};base64,{}", resource.content_type, &resource.data);
                    Some(data_url.trim().to_owned())
                } else {
                    // TODO: handle error - throw?
                    #[cfg(test)]
                    eprintln!("Matched rule with redirect option but did not find corresponding resource to send");
                    None
                }
            } else {
                None
            }
        });

        // If something has already matched before but we don't know what, still return a match
        let matched = exception.is_none() && (filter.is_some() || matched_rule);
        BlockerResult {
            matched,
            explicit_cancel: matched && filter.is_some() && filter.as_ref().map(|f| f.is_explicit_cancel()).unwrap_or_else(|| false),
            important: filter.is_some() && filter.as_ref().map(|f| f.is_important()).unwrap_or_else(|| false),
            redirect,
            exception: exception.as_ref().map(|f| f.to_string()), // copy the exception
            filter: filter.as_ref().map(|f| f.to_string()),       // copy the filter
            error: None,
        }
    }

    /**
     * Given a "main_frame" request, check if some content security policies
     * should be injected in the page.
     */
    pub fn get_csp_directives(&self, _request: Request) -> Option<String> {
        unimplemented!()
    }

    pub fn new(network_filters: Vec<NetworkFilter>, options: &BlockerOptions) -> Blocker {
        // Capacity of filter subsets estimated based on counts in EasyList and EasyPrivacy - if necessary
        // the Vectors will grow beyond the pre-set capacity, but it is more efficient to allocate all at once
        // $csp=
        let mut csp = Vec::with_capacity(200);
        // @@filter
        let mut exceptions = Vec::with_capacity(network_filters.len() / 8);
        // $important
        let mut importants = Vec::with_capacity(200);
        // $redirect
        let mut redirects = Vec::with_capacity(200);
        // $tag=
        let mut tagged_filters_all = Vec::with_capacity(200);
        // $badfilter
        let mut badfilters = Vec::with_capacity(100);
        // $generichide
        let mut generic_hide = Vec::with_capacity(4000);
        // All other filters
        let mut filters = Vec::with_capacity(network_filters.len());

        // Injections
        // TODO: resource handling

        if !network_filters.is_empty() {
            for filter in network_filters.iter() {
                if filter.is_badfilter() {
                    badfilters.push(filter);
                }
            }
            let badfilter_ids: HashSet<Hash> = badfilters.iter().map(|f| f.get_id_without_badfilter()).collect();
            for filter in network_filters {
                // skip any bad filters
                let filter_id = filter.get_id();
                if badfilter_ids.contains(&filter_id) || filter.is_badfilter() {
                    continue;
                }
                if filter.is_csp() {
                    csp.push(filter);
                } else if filter.is_generic_hide() {
                    generic_hide.push(filter);
                } else if filter.is_exception() {
                    exceptions.push(filter);
                } else if filter.is_important() {
                    importants.push(filter);
                } else if filter.is_redirect() {
                    redirects.push(filter);
                } else if filter.tag.is_some() {
                    tagged_filters_all.push(filter);
                } else {
                    filters.push(filter);
                }
            }
        }

        tagged_filters_all.shrink_to_fit();

        Blocker {
            csp: NetworkFilterList::new(csp, options.enable_optimizations),
            exceptions: NetworkFilterList::new(exceptions, options.enable_optimizations),
            importants: NetworkFilterList::new(importants, options.enable_optimizations),
            redirects: NetworkFilterList::new(redirects, options.enable_optimizations),
            filters_tagged: NetworkFilterList::new(Vec::new(), options.enable_optimizations),
            filters: NetworkFilterList::new(filters, options.enable_optimizations),
            generic_hide: NetworkFilterList::new(generic_hide, options.enable_optimizations),
            // Tags special case for enabling/disabling them dynamically
            tags_enabled: HashSet::new(),
            tagged_filters_all,
            hot_filters: NetworkFilterList::default(),
            // Options
            enable_optimizations: options.enable_optimizations,

            resources: RedirectResourceStorage::default(),
            #[cfg(feature = "object-pooling")]
            pool: TokenPool::default(),
        }
    }

    /// If optimizations are enabled, the `Blocker` will be configured to automatically optimize
    /// its filters after batch updates. However, even if they are disabled, it is possible to
    /// manually call `optimize()`. It may be useful to have finer-grained control over
    /// optimization scheduling when frequently updating filters.
    pub fn optimize(&mut self) {
        self.csp.optimize();
        self.exceptions.optimize();
        self.importants.optimize();
        self.redirects.optimize();
        self.filters_tagged.optimize();
        self.filters.optimize();
        self.generic_hide.optimize();
    }

    pub fn filter_exists(&self, filter: &NetworkFilter) -> bool {
        if filter.is_csp() {
            self.csp.filter_exists(filter)
        } else if filter.is_generic_hide() {
            self.generic_hide.filter_exists(filter)
        } else if filter.is_exception() {
            self.exceptions.filter_exists(filter)
        } else if filter.is_important() {
            self.importants.filter_exists(filter)
        } else if filter.is_redirect() {
            self.redirects.filter_exists(filter)
        } else if filter.tag.is_some() {
            self.tagged_filters_all.iter().any(|f| f.id == filter.id)
        } else {
            self.filters.filter_exists(filter)
        }
    }

    pub fn add_filter(&mut self, filter: NetworkFilter) -> Result<(), BlockerError> {
        if filter.is_badfilter() {
            Err(BlockerError::BadFilterAddUnsupported)
        } else if self.filter_exists(&filter) {
            Err(BlockerError::FilterExists)
        } else if filter.is_csp() {
            self.csp.add_filter(filter);
            Ok(())
        } else if filter.is_generic_hide() {
            self.generic_hide.add_filter(filter);
            Ok(())
        } else if filter.is_exception() {
            self.exceptions.add_filter(filter);
            Ok(())
        } else if filter.is_important() {
            self.importants.add_filter(filter);
            Ok(())
        } else if filter.is_redirect() {
            self.redirects.add_filter(filter);
            Ok(())
        } else if filter.tag.is_some() {
            self.tagged_filters_all.push(filter);
            let tags_enabled = HashSet::from_iter(self.tags_enabled().into_iter());
            self.tags_with_set(tags_enabled);
            Ok(())
        } else {
            self.filters.add_filter(filter);
            Ok(())
        }
    }

    pub fn use_tags(&mut self, tags: &[&str]) {
        let tag_set: HashSet<String> = HashSet::from_iter(tags.iter().map(|&t| String::from(t)));
        self.tags_with_set(tag_set);
    }

    pub fn enable_tags(&mut self, tags: &[&str]) {
        let tag_set: HashSet<String> = HashSet::from_iter(tags.iter().map(|&t| String::from(t)))
            .union(&self.tags_enabled)
            .cloned()
            .collect();
        self.tags_with_set(tag_set);
    }

    pub fn disable_tags(&mut self, tags: &[&str]) {
        let tag_set: HashSet<String> = self.tags_enabled
            .difference(&HashSet::from_iter(tags.iter().map(|&t| String::from(t))))
            .cloned()
            .collect();
        self.tags_with_set(tag_set);
    }

    fn tags_with_set(&mut self, tags_enabled: HashSet<String>) {
        self.tags_enabled = tags_enabled;
        let filters: Vec<NetworkFilter> = self.tagged_filters_all.iter()
            .filter(|n| n.tag.is_some() && self.tags_enabled.contains(n.tag.as_ref().unwrap()))
            .cloned()
            .collect();
        self.filters_tagged = NetworkFilterList::new(filters, self.enable_optimizations);
    }

    pub fn tags_enabled(&self) -> Vec<String> {
        self.tags_enabled.iter().cloned().collect()
    }

    pub fn use_resources(&mut self, resources: &[Resource]) {
        let resources = RedirectResourceStorage::from_resources(resources);
        self.resources = resources;
    }

    pub fn add_resource(&mut self, resource: &Resource) {
        self.resources.add_resource(resource);
    }

    pub fn get_resource(&self, key: &str) -> Option<&RedirectResource> {
        self.resources.get_resource(key)
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct NetworkFilterList {
    filter_map: HashMap<Hash, Vec<Arc<NetworkFilter>>>,
}

impl NetworkFilterList {
    pub fn new(filters: Vec<NetworkFilter>, optimize: bool) -> NetworkFilterList {
        // Compute tokens for all filters
        let filter_tokens: Vec<_> = filters
            .into_iter()
            .map(|filter| {
                let tokens = filter.get_tokens();
                (Arc::new(filter), tokens)
            })
            .collect();
        // compute the tokens' frequency histogram
        let (total_number_of_tokens, tokens_histogram) = token_histogram(&filter_tokens);

        // Build a HashMap of tokens to Network Filters (held through Arc, Atomic Reference Counter)
        let mut filter_map = HashMap::with_capacity(filter_tokens.len());
        {
            for (filter_pointer, multi_tokens) in filter_tokens {
                for tokens in multi_tokens {
                    let mut best_token: Hash = 0;
                    let mut min_count = total_number_of_tokens + 1;
                    for token in tokens {
                        match tokens_histogram.get(&token) {
                            None => {
                                min_count = 0;
                                best_token = token
                            }
                            Some(&count) if count < min_count => {
                                min_count = count;
                                best_token = token
                            }
                            _ => {}
                        }
                    }
                    insert_dup(&mut filter_map, best_token, Arc::clone(&filter_pointer));
                }
            }
        }

        let mut self_ = NetworkFilterList {
            filter_map,
        };

        if optimize {
            self_.optimize();
        } else {
            self_.filter_map.shrink_to_fit();
        }

        self_
    }

    pub fn optimize(&mut self) {
        let mut optimized_map = HashMap::with_capacity(self.filter_map.len());
        for (key, filters) in self.filter_map.drain() {
            let mut unoptimized: Vec<NetworkFilter> = Vec::with_capacity(filters.len());
            let mut unoptimizable: Vec<Arc<NetworkFilter>> = Vec::with_capacity(filters.len());
            for f in filters {
                match Arc::try_unwrap(f) {
                    Ok(f) => unoptimized.push(f),
                    Err(af) => unoptimizable.push(af)
                }
            }

            let mut optimized: Vec<_> = if unoptimized.len() > 1 {
                optimizer::optimize(unoptimized).into_iter().map(Arc::new).collect()
            } else {
                // nothing to optimize
                unoptimized.into_iter().map(Arc::new).collect()
            };

            optimized.append(&mut unoptimizable);
            optimized_map.insert(key, optimized);
        }

        // won't mutate anymore, shrink to fit items
        optimized_map.shrink_to_fit();

        self.filter_map = optimized_map;
    }

    pub fn add_filter(&mut self, filter: NetworkFilter) {
        let filter_tokens = filter.get_tokens();
        let total_rules = vec_hashmap_len(&self.filter_map);
        let filter_pointer = Arc::new(filter);

        for tokens in filter_tokens {
            let mut best_token: Hash = 0;
            let mut min_count = total_rules + 1;
            for token in tokens {
                match self.filter_map.get(&token) {
                    None => {
                        min_count = 0;
                        best_token = token
                    }
                    Some(filters) if filters.len() < min_count => {
                        min_count = filters.len();
                        best_token = token
                    }
                    _ => {}
                }
            }

            insert_dup(&mut self.filter_map, best_token, Arc::clone(&filter_pointer));
        }
    }

    pub fn filter_exists(&self, filter: &NetworkFilter) -> bool {
        // if self.optimized == Some(true) {
        //     return Err(BlockerError::OptimizedFilterExistence)
        // }
        let mut tokens: Vec<_> = filter.get_tokens().into_iter().flatten().collect();

        if tokens.is_empty() {
            tokens.push(0)
        }

        for token in tokens {
            if let Some(filters) = self.filter_map.get(&token) {
                for saved_filter in filters {
                    if saved_filter.id == filter.id {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn check(&self, request: &Request, request_tokens: &[Hash], active_tags: &HashSet<String>) -> Option<&NetworkFilter> {
        #[cfg(feature = "metrics")]
        let mut filters_checked = 0;
        #[cfg(feature = "metrics")]
        let mut filter_buckets = 0;

        #[cfg(not(feature = "metrics"))]
        {
            if self.filter_map.is_empty() {
                return None;
            }
        }

        if let Some(source_hostname_hashes) = request.source_hostname_hashes.as_ref() {
            for token in source_hostname_hashes {
                if let Some(filter_bucket) = self.filter_map.get(token) {
                    #[cfg(feature = "metrics")]
                    {
                        filter_buckets += 1;
                    }

                    for filter in filter_bucket {
                        #[cfg(feature = "metrics")]
                        {
                            filters_checked += 1;
                        }
                        // if matched, also needs to be tagged with an active tag (or not tagged at all)
                        if filter.matches(request) && filter.tag.as_ref().map(|t| active_tags.contains(t)).unwrap_or(true) {
                            #[cfg(feature = "metrics")]
                            print!("true\t{}\t{}\tskipped\t{}\t{}\t", filter_buckets, filters_checked, filter_buckets, filters_checked);
                            return Some(filter);
                        }
                    }
                }
            }
        }

        #[cfg(feature = "metrics")]
        print!("false\t{}\t{}\t", filter_buckets, filters_checked);

        for token in request_tokens {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                #[cfg(feature = "metrics")]
                {
                    filter_buckets += 1;
                }
                for filter in filter_bucket {
                    #[cfg(feature = "metrics")]
                    {
                        filters_checked += 1;
                    }
                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request) && filter.tag.as_ref().map(|t| active_tags.contains(t)).unwrap_or(true) {
                        #[cfg(feature = "metrics")]
                        print!("true\t{}\t{}\t", filter_buckets, filters_checked);
                        return Some(filter);
                    }
                }
            }
        }

        #[cfg(feature = "metrics")]
        print!("false\t{}\t{}\t", filter_buckets, filters_checked);

        None
    }
}

fn insert_dup<K, V, H: std::hash::BuildHasher>(map: &mut HashMap<K, Vec<V>, H>, k: K, v: V)
where
    K: std::cmp::Ord + std::hash::Hash,
{
    map.entry(k).or_insert_with(Vec::new).push(v)
}

fn vec_hashmap_len<K: std::cmp::Eq + std::hash::Hash, V, H: std::hash::BuildHasher>(map: &HashMap<K, Vec<V>, H>) -> usize {
    let mut size = 0 as usize;
    for (_, val) in map.iter() {
        size += val.len();
    }
    size
}

fn token_histogram<T>(filter_tokens: &[(T, Vec<Vec<Hash>>)]) -> (u32, HashMap<Hash, u32>) {
    let mut tokens_histogram: HashMap<Hash, u32> = HashMap::new();
    let mut number_of_tokens = 0;
    for (_, tokens) in filter_tokens.iter() {
        for tg in tokens {
            for t in tg {
                *tokens_histogram.entry(*t).or_insert(0) += 1;
                number_of_tokens += 1;
            }
        }
    }

    for bad_token in ["http", "https", "www", "com"].iter() {
        tokens_histogram.insert(fast_hash(bad_token), number_of_tokens);
    }

    (number_of_tokens, tokens_histogram)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_dup_works() {
        let mut dup_map: HashMap<Hash, Vec<String>> = HashMap::new();

        // inserts into empty
        insert_dup(&mut dup_map, 1, String::from("foo"));
        assert_eq!(dup_map.get(&1), Some(&vec![String::from("foo")]));

        // adds item
        insert_dup(&mut dup_map, 1, String::from("bar"));
        assert_eq!(
            dup_map.get(&1),
            Some(&vec![String::from("foo"), String::from("bar")])
        );

        // inserts into another key item
        insert_dup(&mut dup_map, 123, String::from("baz"));
        assert_eq!(dup_map.get(&123), Some(&vec![String::from("baz")]));
        assert_eq!(
            dup_map.get(&1),
            Some(&vec![String::from("foo"), String::from("bar")])
        );
    }

    #[test]
    fn token_histogram_works() {
        // handle the case of just 1 token
        {
            let tokens = vec![(0, vec![vec![111]])];
            let (total_tokens, histogram) = token_histogram(&tokens);
            assert_eq!(total_tokens, 1);
            assert_eq!(histogram.get(&111), Some(&1));
            // include bad tokens
            assert_eq!(histogram.get(&fast_hash("http")), Some(&1));
            assert_eq!(histogram.get(&fast_hash("www")), Some(&1));
        }

        // handle the case of repeating tokens
        {
            let tokens = vec![(0, vec![vec![111]]), (1, vec![vec![111]])];
            let (total_tokens, histogram) = token_histogram(&tokens);
            assert_eq!(total_tokens, 2);
            assert_eq!(histogram.get(&111), Some(&2));
            // include bad tokens
            assert_eq!(histogram.get(&fast_hash("http")), Some(&2));
            assert_eq!(histogram.get(&fast_hash("www")), Some(&2));
        }

        // handle the different token set sizes
        {
            let tokens = vec![
                (0, vec![vec![111, 123, 132]]),
                (1, vec![vec![111], vec![123], vec![132]]),
                (2, vec![vec![111, 123], vec![132]]),
                (3, vec![vec![111, 111], vec![111]]),
            ];
            let (total_tokens, histogram) = token_histogram(&tokens);
            assert_eq!(total_tokens, 12);
            assert_eq!(histogram.get(&111), Some(&6));
            assert_eq!(histogram.get(&123), Some(&3));
            assert_eq!(histogram.get(&132), Some(&3));
            // include bad tokens
            assert_eq!(histogram.get(&fast_hash("http")), Some(&12));
            assert_eq!(histogram.get(&fast_hash("www")), Some(&12));
        }
    }

    #[test]
    fn network_filter_list_new_works() {
        {
            let filters = vec!["||foo.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            let maybe_matching_filter = filter_list.filter_map.get(&fast_hash("foo"));
            assert!(maybe_matching_filter.is_some(), "Expected filter not found");
        }
        // choses least frequent token
        {
            let filters = vec!["||foo.com", "||bar.com/foo"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            assert_eq!(
                filter_list.filter_map.get(&fast_hash("bar")).unwrap().len(),
                1
            );
            assert_eq!(
                filter_list.filter_map.get(&fast_hash("foo")).unwrap().len(),
                1
            );
        }
        // choses blacklisted token when no other choice
        {
            let filters = vec!["||foo.com", "||foo.com/bar", "||www"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            assert!(
                filter_list.filter_map.get(&fast_hash("www")).is_some(),
                "Filter matching {} not found",
                "www"
            );
            assert_eq!(
                filter_list.filter_map.get(&fast_hash("www")).unwrap().len(),
                1
            );
        }
        // uses domain as token when only one domain
        {
            let filters = vec!["||foo.com", "||foo.com$domain=bar.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            assert!(
                filter_list.filter_map.get(&fast_hash("bar.com")).is_some(),
                "Filter matching {} not found",
                "bar.com"
            );
            assert_eq!(
                filter_list
                    .filter_map
                    .get(&fast_hash("bar.com"))
                    .unwrap()
                    .len(),
                1
            );
        }
        // dispatches filter to multiple buckets per domain options if no token in main part
        {
            let filters = vec!["foo*$domain=bar.com|baz.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            assert_eq!(filter_list.filter_map.len(), 2);
            assert!(
                filter_list.filter_map.get(&fast_hash("bar.com")).is_some(),
                "Filter matching {} not found",
                "bar.com"
            );
            assert_eq!(
                filter_list
                    .filter_map
                    .get(&fast_hash("bar.com"))
                    .unwrap()
                    .len(),
                1
            );
            assert!(
                filter_list.filter_map.get(&fast_hash("baz.com")).is_some(),
                "Filter matching {} not found",
                "baz.com"
            );
            assert_eq!(
                filter_list
                    .filter_map
                    .get(&fast_hash("baz.com"))
                    .unwrap()
                    .len(),
                1
            );
        }
    }

    fn test_requests_filters(filters: &Vec<&str>, requests: &Vec<(Request, bool)>) {
        let network_filters: Vec<_> = filters
            .into_iter()
            .map(|f| NetworkFilter::parse(&f, true))
            .filter_map(Result::ok)
            .collect();
        let filter_list = NetworkFilterList::new(network_filters, false);

        requests.into_iter().for_each(|(req, expected_result)| {
            let mut tokens = Vec::new();
            req.get_tokens(&mut tokens);
            let matched_rule = filter_list.check(&req, &tokens, &HashSet::new());
            if *expected_result {
                assert!(matched_rule.is_some(), "Expected match for {}", req.url);
            } else {
                assert!(matched_rule.is_none(), "Expected no match for {}, matched with {}", req.url, matched_rule.unwrap().to_string());
            }
        });
    }

    #[test]
    fn network_filter_list_check_works_plain_filter() {
        // includes cases with fall back to 0 bucket (no tokens from a rule)
        let filters = vec![
            "foo",
            "-foo-",
            "&fo.o=+_-",
            "foo/bar/baz",
            "com/bar/baz",
            "https://bar.com/bar/baz",
        ];

        let url_results = vec![
            ("https://bar.com/foo", true),
            ("https://bar.com/baz/foo", true),
            ("https://bar.com/q=foo/baz", true),
            ("https://foo.com", true),
            ("https://bar.com/baz/42-foo-q", true),
            ("https://bar.com?baz=42&fo.o=+_-", true),
            ("https://bar.com/foo/bar/baz", true),
            ("https://bar.com/bar/baz", true),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::from_url(url).unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_fuzzy_filter() {
        let filters = vec![
            "f$fuzzy",
            "foo$fuzzy",
            "foo/bar$fuzzy",
            "foo bar$fuzzy",
            "foo bar baz$fuzzy",
            "coo car caz 42$fuzzy",
        ];

        let url_results = vec![
            ("https://bar.com/f", true),
            ("https://bar.com/foo", true),
            ("https://bar.com/foo/baz", true),
            ("http://bar.foo.baz", true),
            ("http://car.coo.caz.43", false),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::from_url(url).unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_hostname_anchor() {
        let filters = vec![
            "||foo.com",
            "||bar.com/bar",
            "||coo.baz.",
            "||foo.bar.com^",
            "||foo.baz^",
        ];

        let url_results = vec![
            ("https://foo.com/bar", true),
            ("https://bar.com/bar", true),
            ("https://baz.com/bar", false),
            ("https://baz.foo.com/bar", true),
            ("https://coo.baz.com/bar", true),
            ("https://foo.bar.com/bar", true),
            ("https://foo.baz.com/bar", false),
            ("https://baz.com", false),
            ("https://foo-bar.baz.com/bar", false),
            ("https://foo.de", false),
            ("https://bar.foo.de", false),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::from_url(url).unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_unicode() {
        let filters = vec![
            "||firstrowsports.li/frame/",
            "||fırstrowsports.eu/pu/",
            "||atđhe.net/pu/",
        ];

        let url_results = vec![
            (
                Request::from_url("https://firstrowsports.li/frame/bar").unwrap(),
                true,
            ),
            (
                Request::from_url("https://secondrowsports.li/frame/bar").unwrap(),
                false,
            ),
            (
                Request::from_url("https://fırstrowsports.eu/pu/foo").unwrap(),
                true,
            ),
            (
                Request::from_url("https://xn--frstrowsports-39b.eu/pu/foo").unwrap(),
                true,
            ),
            (
                Request::from_url("https://atđhe.net/pu/foo").unwrap(),
                true,
            ),
            (
                Request::from_url("https://xn--athe-1ua.net/pu/foo").unwrap(),
                true,
            ),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(request, expected_result)| (request, expected_result))
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_regex_escaping() {
        let filters = vec![
            r#"/^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/$domain=123movies.com|1337x.to"#,
            r#"/\:\/\/data.*\.com\/[a-zA-Z0-9]{30,}/$third-party,xmlhttprequest"#
        ];

        let url_results = vec![
            (
                Request::from_urls("https://bit.ly/bar/", "http://123movies.com", "").unwrap(),
                true,
            ),
            (
                Request::from_urls(
                    "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer",
                    "http://123movies.com",
                    "xmlhttprequest",
                )
                .unwrap(),
                true,
            ),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(request, expected_result)| (request, expected_result))
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }
}

#[cfg(test)]
mod blocker_tests {

    use super::*;
    use crate::lists::{parse_filters, FilterFormat};
    use crate::request::Request;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    fn test_requests_filters(filters: &[String], requests: &[(Request, bool)]) {
        let (network_filters, _) = parse_filters(filters, true, FilterFormat::Standard);

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        requests.iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if *expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn badfilter_does_not_match() {
        let filters = vec![
            String::from("||foo.com$badfilter")
        ];
        let url_results = vec![
            (
                Request::from_urls("https://foo.com", "https://bar.com", "image").unwrap(),
                false,
            ),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(request, expected_result)| (request, expected_result))
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn badfilter_cancels_with_same_id() {
        let filters = vec![
            String::from("||foo.com$domain=bar.com|foo.com,badfilter"),
            String::from("||foo.com$domain=foo.com|bar.com")
        ];
        let url_results = vec![
            (
                Request::from_urls("https://foo.com", "https://bar.com", "image").unwrap(),
                false,
            ),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(request, expected_result)| (request, expected_result))
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn badfilter_does_not_cancel_similar_filter() {
        let filters = vec![
            String::from("||foo.com$domain=bar.com|foo.com,badfilter"),
            String::from("||foo.com$domain=foo.com|bar.com,image")
        ];
        let url_results = vec![
            (
                Request::from_urls("https://foo.com", "https://bar.com", "image").unwrap(),
                true,
            ),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(request, expected_result)| (request, expected_result))
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn hostname_regex_filter_works() {
        let filters = vec![
            String::from("||alimc*.top^$domain=letv.com"),
            String::from("||aa*.top^$domain=letv.com")
        ];
        let url_results = vec![
            (Request::from_urls("https://r.alimc1.top/test.js", "https://minisite.letv.com/", "script").unwrap(), true),
            (Request::from_urls("https://www.baidu.com/test.js", "https://minisite.letv.com/", "script").unwrap(), false),
            (Request::from_urls("https://r.aabb.top/test.js", "https://example.com/", "script").unwrap(), false),
            (Request::from_urls("https://r.aabb.top/test.js", "https://minisite.letv.com/", "script").unwrap(), true),
        ];

        let (network_filters, _) = parse_filters(&filters, true, FilterFormat::Standard);

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }


    #[test]
    fn tags_enable_works() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            (Request::from_url("http://example.com/advert.html").unwrap(), true),
            (Request::from_url("http://example.com/somelongpath/test/2.html").unwrap(), true),
            (Request::from_url("https://brianbondy.com/about").unwrap(), false),
            (Request::from_url("https://brave.com/about").unwrap(), false),
        ];

        let (network_filters, _) = parse_filters(&filters, true, FilterFormat::Standard);

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        blocker.enable_tags(&["stuff"]);
        assert_eq!(blocker.tags_enabled, HashSet::from_iter(vec![String::from("stuff")].into_iter()));
        assert_eq!(vec_hashmap_len(&blocker.filters_tagged.filter_map), 2);

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn tags_enable_adds_tags() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            (Request::from_url("http://example.com/advert.html").unwrap(), true),
            (Request::from_url("http://example.com/somelongpath/test/2.html").unwrap(), true),
            (Request::from_url("https://brianbondy.com/about").unwrap(), true),
            (Request::from_url("https://brave.com/about").unwrap(), true),
        ];

        let (network_filters, _) = parse_filters(&filters, true, FilterFormat::Standard);

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        blocker.enable_tags(&["stuff"]);
        blocker.enable_tags(&["brian"]);
        assert_eq!(blocker.tags_enabled, HashSet::from_iter(vec![String::from("brian"), String::from("stuff")].into_iter()));
        assert_eq!(vec_hashmap_len(&blocker.filters_tagged.filter_map), 4);

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn tags_disable_works() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            (Request::from_url("http://example.com/advert.html").unwrap(), false),
            (Request::from_url("http://example.com/somelongpath/test/2.html").unwrap(), false),
            (Request::from_url("https://brianbondy.com/about").unwrap(), true),
            (Request::from_url("https://brave.com/about").unwrap(), true),
        ];

        let (network_filters, _) = parse_filters(&filters, true, FilterFormat::Standard);

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        blocker.enable_tags(&["brian", "stuff"]);
        assert_eq!(blocker.tags_enabled, HashSet::from_iter(vec![String::from("brian"), String::from("stuff")].into_iter()));
        assert_eq!(vec_hashmap_len(&blocker.filters_tagged.filter_map), 4);
        blocker.disable_tags(&["stuff"]);
        assert_eq!(blocker.tags_enabled, HashSet::from_iter(vec![String::from("brian")].into_iter()));
        assert_eq!(vec_hashmap_len(&blocker.filters_tagged.filter_map), 2);

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn filter_add_badfilter_error() {
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,
        };

        let mut blocker = Blocker::new(Vec::new(), &blocker_options);

        let filter = NetworkFilter::parse("adv$badfilter", true).unwrap();
        let added = blocker.add_filter(filter);
        assert!(added.is_err());
        assert_eq!(added.err().unwrap(), BlockerError::BadFilterAddUnsupported);
    }

    #[test]
    #[ignore]
    fn filter_add_twice_handling_error() {
        {
            // Not allow filter to be added twice hwn the engine is not optimised
            let blocker_options: BlockerOptions = BlockerOptions {
                enable_optimizations: false,
            };

            let mut blocker = Blocker::new(Vec::new(), &blocker_options);

            let filter = NetworkFilter::parse("adv", true).unwrap();
            blocker.add_filter(filter.clone()).unwrap();
            assert!(blocker.filter_exists(&filter), "Expected filter to be inserted");
            let added = blocker.add_filter(filter);
            assert!(added.is_err(), "Expected repeated insertion to fail");
            assert_eq!(added.err().unwrap(), BlockerError::FilterExists, "Expected specific error on repeated insertion fail");
        }
        {
            // Allow filter to be added twice when the engine is optimised
            let blocker_options: BlockerOptions = BlockerOptions {
                enable_optimizations: true,
            };

            let mut blocker = Blocker::new(Vec::new(), &blocker_options);

            let filter = NetworkFilter::parse("adv", true).unwrap();
            blocker.add_filter(filter.clone()).unwrap();
            let added = blocker.add_filter(filter);
            assert!(added.is_ok());
        }
    }

    #[test]
    fn filter_add_tagged() {
        // Allow filter to be added twice when the engine is optimised
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: true,
        };

        let mut blocker = Blocker::new(Vec::new(), &blocker_options);
        blocker.enable_tags(&["brian"]);

        blocker.add_filter(NetworkFilter::parse("adv$tag=stuff", true).unwrap()).unwrap();
        blocker.add_filter(NetworkFilter::parse("somelongpath/test$tag=stuff", true).unwrap()).unwrap();
        blocker.add_filter(NetworkFilter::parse("||brianbondy.com/$tag=brian", true).unwrap()).unwrap();
        blocker.add_filter(NetworkFilter::parse("||brave.com$tag=brian", true).unwrap()).unwrap();

        let url_results = vec![
            (Request::from_url("http://example.com/advert.html").unwrap(), false),
            (Request::from_url("http://example.com/somelongpath/test/2.html").unwrap(), false),
            (Request::from_url("https://brianbondy.com/about").unwrap(), true),
            (Request::from_url("https://brave.com/about").unwrap(), true),
        ];

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", req.url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn exception_force_check() {
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: true,
        };

        let mut blocker = Blocker::new(Vec::new(), &blocker_options);

        blocker.add_filter(NetworkFilter::parse("@@*ad_banner.png", true).unwrap()).unwrap();

        let request = Request::from_url("http://example.com/ad_banner.png").unwrap();

        let matched_rule = blocker.check_parameterised(&request, false, true);
        assert!(!matched_rule.matched);
        assert!(matched_rule.exception.is_some());
    }

    #[test]
    fn generichide() {
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: true,
        };

        let mut blocker = Blocker::new(Vec::new(), &blocker_options);

        blocker.add_filter(NetworkFilter::parse("@@||example.com$generichide", true).unwrap()).unwrap();

        assert!(blocker.check_generic_hide(&Request::from_url("https://example.com").unwrap()));
    }
}

#[cfg(test)]
mod legacy_rule_parsing_tests {
    use crate::utils::rules_from_lists;
    use crate::lists::{parse_filters, FilterFormat};
    use crate::blocker::{Blocker, BlockerOptions};
    use crate::blocker::vec_hashmap_len;

    struct ListCounts {
        pub filters: usize,
        pub cosmetic_filters: usize,
        pub exceptions: usize
    }

    impl std::ops::Add<ListCounts> for ListCounts {
        type Output = ListCounts;

        fn add(self, other: ListCounts) -> Self::Output {
            ListCounts {
                filters: self.filters + other.filters,
                cosmetic_filters: self.cosmetic_filters + other.cosmetic_filters,
                exceptions: self.exceptions + other.exceptions,
            }
        }
    }

    // number of expected EasyList cosmetic rules from old engine is 31144, but is incorrect as it skips a few particularly long rules that are nevertheless valid
    // easyList = { 24478, 31144, 0, 5589 };
    // not handling (and not including) filters with the following options:
    // - $popup
    // - $document
    // - $elemhide
    // difference from original counts caused by not handling document/subdocument options and possibly miscounting on the blocker side.
    // Printing all non-cosmetic, non-html, non-comment/-empty rules and ones with no unsupported options yields 29142 items
    // This engine also handles 3 rules that old one does not
    const EASY_LIST: ListCounts = ListCounts { filters: 24062+3, cosmetic_filters: 31163, exceptions: 5800 };
    // easyPrivacy = { 11817, 0, 0, 1020 };
    // differences in counts explained by hashset size underreporting as detailed in the next two cases
    const EASY_PRIVACY: ListCounts = ListCounts { filters: 11889, cosmetic_filters: 0, exceptions: 1021 };
    // ublockUnbreak = { 4, 8, 0, 94 };
    // differences in counts explained by client.hostAnchoredExceptionHashSet->GetSize() underreporting when compared to client.numHostAnchoredExceptionFilters
    const UBLOCK_UNBREAK: ListCounts = ListCounts { filters: 4, cosmetic_filters: 8, exceptions: 98 };
    // braveUnbreak = { 31, 0, 0, 4 };
    // differences in counts explained by client.hostAnchoredHashSet->GetSize() underreporting when compared to client.numHostAnchoredFilters
    const BRAVE_UNBREAK: ListCounts = ListCounts { filters: 32, cosmetic_filters: 0, exceptions: 4 };
    // disconnectSimpleMalware = { 2450, 0, 0, 0 };
    const DISCONNECT_SIMPLE_MALWARE: ListCounts = ListCounts { filters: 2450, cosmetic_filters: 0, exceptions: 0 };
    // spam404MainBlacklist = { 5629, 166, 0, 0 };
    const SPAM_404_MAIN_BLACKLIST: ListCounts = ListCounts { filters: 5629, cosmetic_filters: 166, exceptions: 0 };
    const MALWARE_DOMAIN_LIST: ListCounts = ListCounts { filters: 1104, cosmetic_filters: 0, exceptions: 0 };
    const MALWARE_DOMAINS: ListCounts = ListCounts { filters: 26853, cosmetic_filters: 0, exceptions: 0 };

    fn check_list_counts(rule_lists: &[String], format: FilterFormat, expectation: ListCounts) {
        let rules = rules_from_lists(rule_lists);

        let (network_filters, cosmetic_filters) = parse_filters(&rules, true, format);

        assert_eq!(
            (network_filters.len(),
            network_filters.iter().filter(|f| f.is_exception()).count(),
            cosmetic_filters.len()),
            (expectation.filters + expectation.exceptions,
            expectation.exceptions,
            expectation.cosmetic_filters),
            "Number of collected filters does not match expectation");

        let blocker_options = BlockerOptions {
            enable_optimizations: false,    // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        // Some filters in the filter_map are pointed at by multiple tokens, increasing the total number of items
        assert!(vec_hashmap_len(&blocker.exceptions.filter_map) + vec_hashmap_len(&blocker.generic_hide.filter_map)
            >= expectation.exceptions, "Number of collected exceptions does not match expectation");

        assert!(vec_hashmap_len(&blocker.filters.filter_map) +
            vec_hashmap_len(&blocker.importants.filter_map) +
            vec_hashmap_len(&blocker.redirects.filter_map) +
            vec_hashmap_len(&blocker.csp.filter_map) >=
            expectation.filters, "Number of collected network filters does not match expectation");
    }

    #[test]
    fn parse_easylist() {
        check_list_counts(&vec![String::from("./data/test/easylist.txt")], FilterFormat::Standard, EASY_LIST);
    }

    #[test]
    fn parse_easyprivacy() {
        check_list_counts(&vec![String::from("./data/test/easyprivacy.txt")], FilterFormat::Standard, EASY_PRIVACY);
    }

    #[test]
    fn parse_ublock_unbreak() {
        check_list_counts(&vec![String::from("./data/test/ublock-unbreak.txt")], FilterFormat::Standard, UBLOCK_UNBREAK);
    }

    #[test]
    fn parse_brave_unbreak() {
        check_list_counts(&vec![String::from("./data/test/brave-unbreak.txt")], FilterFormat::Standard, BRAVE_UNBREAK);
    }

    #[test]
    fn parse_brave_disconnect_simple_malware() {
        check_list_counts(&vec![String::from("./data/test/disconnect-simple-malware.txt")], FilterFormat::Standard, DISCONNECT_SIMPLE_MALWARE);
    }

    #[test]
    fn parse_spam404_main_blacklist() {
        check_list_counts(&vec![String::from("./data/test/spam404-main-blacklist.txt")], FilterFormat::Standard, SPAM_404_MAIN_BLACKLIST);
    }

    #[test]
    fn parse_malware_domain_list() {
        check_list_counts(&vec![String::from("./data/test/malwaredomainlist.txt")], FilterFormat::Hosts, MALWARE_DOMAIN_LIST);
    }

    #[test]
    fn parse_malware_domain_list_just_hosts() {
        check_list_counts(&vec![String::from("./data/test/malwaredomainlist_justhosts.txt")], FilterFormat::Hosts, MALWARE_DOMAIN_LIST);
    }

    #[test]
    fn parse_malware_domains() {
        check_list_counts(&vec![String::from("./data/test/malwaredomains.txt")], FilterFormat::Hosts, MALWARE_DOMAINS);
    }

    #[test]
    fn parse_multilist() {
        let expectation = EASY_LIST + EASY_PRIVACY + UBLOCK_UNBREAK + BRAVE_UNBREAK;
        check_list_counts(
            &vec![
                String::from("./data/test/easylist.txt"),
                String::from("./data/test/easyprivacy.txt"),
                String::from("./data/test/ublock-unbreak.txt"),
                String::from("./data/test/brave-unbreak.txt"),
            ],
            FilterFormat::Standard,
            expectation,
        )
    }

    #[test]
    fn parse_malware_multilist() {
        let expectation = SPAM_404_MAIN_BLACKLIST + DISCONNECT_SIMPLE_MALWARE;
        check_list_counts(
            &vec![
                String::from("./data/test/spam404-main-blacklist.txt"),
                String::from("./data/test/disconnect-simple-malware.txt"),
            ],
            FilterFormat::Standard,
            expectation,
        )
    }

    #[test]
    fn parse_hosts_formats() {
        let expectation = MALWARE_DOMAIN_LIST + MALWARE_DOMAINS;
        check_list_counts(
            &vec![
                String::from("./data/test/malwaredomainlist.txt"),
                String::from("./data/test/malwaredomains.txt"),
            ],
            FilterFormat::Hosts,
            expectation,
        )
    }
}
