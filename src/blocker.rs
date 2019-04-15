use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::filters::network::NetworkFilter;
use crate::request::Request;
use crate::utils::{fast_hash, Hash};
use crate::optimizer;

pub struct BlockerOptions {
    pub debug: bool,
    pub enable_optimizations: bool,
    pub load_cosmetic_filters: bool,
    pub load_network_filters: bool,
}

pub struct BlockerResult {
    pub matched: bool,
    pub redirect: Option<String>,
    pub exception: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum BlockerError {
    SerializationError,
    DeserializationError
}

#[derive(Serialize, Deserialize)]
pub struct Blocker {
    csp: NetworkFilterList,
    exceptions: NetworkFilterList,
    importants: NetworkFilterList,
    redirects: NetworkFilterList,
    filters: NetworkFilterList,

    debug: bool,
    enable_optimizations: bool,
    load_cosmetic_filters: bool,
    load_network_filters: bool,
}

impl Blocker {
    /**
     * Decide if a network request (usually from WebRequest API) should be
     * blocked, redirected or allowed.
     */
    pub fn check(&self, request: &Request) -> BlockerResult {
        if !self.load_network_filters || !request.is_supported {
            return BlockerResult {
                matched: false,
                redirect: None,
                exception: None,
                filter: None,
            };
        }

        // Check the filters in the following order:
        // 1. $important (not subject to exceptions)
        // 2. redirection ($redirect=resource)
        // 3. normal filters
        // 4. exceptions
        let filter = self
            .importants
            .check(request)
            .or_else(|| self.redirects.check(request))
            .or_else(|| self.filters.check(request));

        let exception = filter.as_ref().and_then(|f| {
            // Set `bug` of request
            // TODO - avoid mutability
            if !f.is_important() {
                if f.has_bug() {
                    let mut request_bug = request.clone();
                    request_bug.bug = f.bug;
                    self.exceptions.check(&request_bug)
                } else {
                    self.exceptions.check(request)
                }
            } else {
                None
            }
        });

        // If there is a match
        let redirect: Option<String> = filter.as_ref().and_then(|f| {
            if f.is_redirect() {
                // TODO: build up redirect URL from matching resource
                unimplemented!()
            } else {
                None
            }
        });

        BlockerResult {
            matched: exception.is_none() && filter.is_some(),
            redirect,
            exception: exception.as_ref().map(|f| f.to_string()), // copy the exception
            filter: filter.as_ref().map(|f| f.to_string()),       // copy the filter
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
        // $csp=
        let mut csp = Vec::with_capacity(network_filters.len());
        // @@filter
        let mut exceptions = Vec::with_capacity(network_filters.len());
        // $important
        let mut importants = Vec::with_capacity(network_filters.len());
        // $redirect
        let mut redirects = Vec::with_capacity(network_filters.len());
        // All other filters
        let mut filters = Vec::with_capacity(network_filters.len());

        // Injections
        // TODO: resource handling

        if !network_filters.is_empty() && options.load_network_filters {
            for filter in network_filters {
                if filter.is_csp() {
                    csp.push(filter);
                } else if filter.is_exception() {
                    exceptions.push(filter);
                } else if filter.is_important() {
                    importants.push(filter);
                } else if filter.is_redirect() {
                    redirects.push(filter);
                } else {
                    filters.push(filter);
                }
            }
        }

        csp.shrink_to_fit();
        exceptions.shrink_to_fit();
        importants.shrink_to_fit();
        redirects.shrink_to_fit();
        filters.shrink_to_fit();

        Blocker {
            csp: NetworkFilterList::new(csp, options.enable_optimizations),
            exceptions: NetworkFilterList::new(exceptions, options.enable_optimizations),
            importants: NetworkFilterList::new(importants, options.enable_optimizations),
            redirects: NetworkFilterList::new(redirects, options.enable_optimizations),
            filters: NetworkFilterList::new(filters, options.enable_optimizations),
            // Options
            debug: options.debug,
            enable_optimizations: options.enable_optimizations,
            load_cosmetic_filters: options.load_cosmetic_filters,
            load_network_filters: options.load_network_filters,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NetworkFilterList {
    // A faster structure is possible, but tests didn't indicate much of a difference
    // for different HashMap implementations bulk of the cost in matching
    filter_map: HashMap<Hash, Vec<Arc<NetworkFilter>>>,
}

impl NetworkFilterList {
    pub fn new(filters: Vec<NetworkFilter>, enable_optimizations: bool) -> NetworkFilterList {
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
        let mut filter_map: HashMap<Hash, Vec<Arc<NetworkFilter>>> = HashMap::with_capacity(filter_tokens.len());
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

        // Update all values
        if enable_optimizations {
            let mut optimized_map: HashMap<Hash, Vec<Arc<NetworkFilter>>> = HashMap::with_capacity(filter_map.len());
            for (key, filters) in filter_map {
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

            NetworkFilterList { filter_map: optimized_map }
        } else {
            filter_map.shrink_to_fit();
            NetworkFilterList { filter_map }
        }
    }

    pub fn check(&self, request: &Request) -> Option<&NetworkFilter> {
        let mut request_tokens = request.get_tokens();
        request_tokens.push(0); // add 0 token as the fallback
        
        for token in request_tokens {
            let maybe_filter_bucket = self.filter_map.get(&token);
            if let Some(filter_bucket) = maybe_filter_bucket {
                for filter in filter_bucket {
                    if filter.matches(request) {
                        return Some(filter);
                    }
                }
            }
        }

        None
    }
}

fn insert_dup<K, V>(map: &mut HashMap<K, Vec<V>>, k: K, v: V)
where
    K: std::cmp::Ord + std::hash::Hash,
{
    map.entry(k).or_insert_with(Vec::new).push(v)
}

fn vec_hashmap_len<K: std::cmp::Eq + std::hash::Hash, V>(map: &HashMap<K, Vec<V>>) -> usize {
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
            let matched_rule = filter_list.check(&req);
            if *expected_result {
                assert!(matched_rule.is_some(), "Expected match for {}", req.url);
            } else {
                assert!(matched_rule.is_none(), "Expected no match for {}", req.url);
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

mod legacy_rule_parsing_tests {
    use crate::utils::rules_from_lists;
    use crate::lists::parse_filters;
    use crate::blocker::{Blocker, BlockerOptions};
    use crate::blocker::vec_hashmap_len;

    struct ListCounts {
        pub filters: usize,
        pub cosmetic_filters: usize,
        pub exceptions: usize
    }

    // number of expected EasyList cosmetic rules from old engine is 31144, but is incorrect as it skips a few particularly long rules that are nevertheless valid
    // easyList = { 24478, 31144, 0, 5589 };
    // not handling (and not including) filters with the following options: 
    // - $popup
    // - $generichide
    // - $subdocument
    // - $document
    // - $elemhide
    // difference from original counts caused by not handling document/subdocument options and possibly miscounting on the blocker side.
    // Printing all non-cosmetic, non-html, non-comment/-empty rules and ones with no unsupported options yields 29142 items
    // This engine also handles 3 rules that old one does not
    const EASY_LIST: ListCounts = ListCounts { filters: 24062+3, cosmetic_filters: 31163, exceptions: 5080-23 };
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

    fn check_list_counts(rule_lists: &Vec<String>, expectation: ListCounts) {
        let rules = rules_from_lists(rule_lists);
        
        // load_network_filters = true, load)cosmetic_filters = true, debug = true
        let (network_filters, cosmetic_filters) = parse_filters(&rules, true, true, true); 

        assert_eq!(
            (network_filters.len(),
            network_filters.iter().filter(|f| f.is_exception()).count(),
            cosmetic_filters.len()),
            (expectation.filters + expectation.exceptions,
            expectation.exceptions,
            expectation.cosmetic_filters),
            "Number of collected filters does not match expectation");
        
        let blocker_options = BlockerOptions {
            debug: false,
            enable_optimizations: false,    // optimizations will reduce number of rules
            load_cosmetic_filters: false,   
            load_network_filters: true
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        // Some filters in the filter_map are pointed at by multiple tokens, increasing the total number of items
        assert!(vec_hashmap_len(&blocker.exceptions.filter_map) >= expectation.exceptions, "Number of collected exceptions does not match expectation");

        assert!(vec_hashmap_len(&blocker.filters.filter_map) + 
            vec_hashmap_len(&blocker.importants.filter_map) +
            vec_hashmap_len(&blocker.redirects.filter_map) +
            vec_hashmap_len(&blocker.csp.filter_map) >=
            expectation.filters, "Number of collected network filters does not match expectation");
    }

    #[test]
    fn parse_easylist() {
        check_list_counts(&vec![String::from("./data/test/easylist.txt")], EASY_LIST);
    }

    #[test]
    fn parse_easyprivacy() {
        check_list_counts(&vec![String::from("./data/test/easyprivacy.txt")], EASY_PRIVACY);
    }

    #[test]
    fn parse_ublock_unbreak() {
        check_list_counts(&vec![String::from("./data/test/ublock-unbreak.txt")], UBLOCK_UNBREAK);
    }

    #[test]
    fn parse_brave_unbreak() {
        check_list_counts(&vec![String::from("./data/test/brave-unbreak.txt")], BRAVE_UNBREAK);
    }

    #[test]
    fn parse_brave_disconnect_simple_malware() {
        check_list_counts(&vec![String::from("./data/test/disconnect-simple-malware.txt")], DISCONNECT_SIMPLE_MALWARE);
    }

    #[test]
    fn parse_spam404_main_blacklist() {
        check_list_counts(&vec![String::from("./data/test/spam404-main-blacklist.txt")], SPAM_404_MAIN_BLACKLIST);
    }

    #[test]
    fn parse_multilist() {
        let expectation = ListCounts {
            filters: EASY_LIST.filters + EASY_PRIVACY.filters + UBLOCK_UNBREAK.filters + BRAVE_UNBREAK.filters,
            cosmetic_filters: EASY_LIST.cosmetic_filters + EASY_PRIVACY.cosmetic_filters + UBLOCK_UNBREAK.cosmetic_filters + BRAVE_UNBREAK.cosmetic_filters,
            exceptions: EASY_LIST.exceptions + EASY_PRIVACY.exceptions + UBLOCK_UNBREAK.exceptions + BRAVE_UNBREAK.exceptions
        };
        check_list_counts(&vec![
            String::from("./data/test/easylist.txt"),
            String::from("./data/test/easyprivacy.txt"),
            String::from("./data/test/ublock-unbreak.txt"),
            String::from("./data/test/brave-unbreak.txt"),
        ], expectation)
    }

    #[test]
    fn parse_malware_multilist() {
        let expectation = ListCounts {
            filters: SPAM_404_MAIN_BLACKLIST.filters + DISCONNECT_SIMPLE_MALWARE.filters,
            cosmetic_filters: SPAM_404_MAIN_BLACKLIST.cosmetic_filters + DISCONNECT_SIMPLE_MALWARE.cosmetic_filters,
            exceptions: SPAM_404_MAIN_BLACKLIST.exceptions + DISCONNECT_SIMPLE_MALWARE.exceptions,
        };
        check_list_counts(&vec![
            String::from("./data/test/spam404-main-blacklist.txt"),
            String::from("./data/test/disconnect-simple-malware.txt"),
        ], expectation)
    }
}