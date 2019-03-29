use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

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
            // if f.has_bug() {
            //     request.bug = f.bug;
            // }
            self.exceptions.check(request)
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
            redirect: redirect,
            exception: exception.as_ref().map(|f| f.to_string()), // copy the exception
            filter: filter.as_ref().map(|f| f.to_string()),       // copy the filter
        }
    }

    /**
     * Given a "main_frame" request, check if some content security policies
     * should be injected in the page.
     */
    pub fn get_csp_directives(&self, request: Request) -> Option<String> {
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

        if network_filters.len() > 0 && options.load_network_filters {
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

use std::cell::RefCell;

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

                let mut optimized: Vec<_>;
                if unoptimized.len() > 1 {
                    optimized = optimizer::optimize(unoptimized).into_iter().map(|f| Arc::new(f)).collect();
                } else {
                    // nothing to optimize
                    optimized = unoptimized.into_iter().map(|f| Arc::new(f)).collect();
                }
                
                optimized.append(&mut unoptimizable);
                optimized_map.insert(key, optimized);
            }

            // won't mutate anymore, shrink to fit items
            optimized_map.shrink_to_fit();

            NetworkFilterList { filter_map: optimized_map }
        } else {

            // for (_, filters) in filter_map.iter_mut() {
            //     filters.sort_by(|a, b| a.get_cost().partial_cmp(&b.get_cost()).unwrap())
            // }

            filter_map.shrink_to_fit();
            NetworkFilterList { filter_map: filter_map }
        }
    }

    pub fn check(&self, request: &Request) -> Option<&NetworkFilter> {
        let mut request_tokens = request.get_tokens();
        request_tokens.push(0); // add 0 token as the fallback
        
        // let mut tokens_checked = 0;
        // let mut filters_checked = 0;
        for token in request_tokens {
            // tokens_checked += 1;
            let maybe_filter_bucket = self.filter_map.get(&token);
            for filter_bucket in maybe_filter_bucket {
                for filter in filter_bucket {
                    // filters_checked += 1;
                    if filter.matches(request) {
                        // println!("Filters checked MATCH : {} in {} buckets", filters_checked, tokens_checked);
                        return Some(filter);
                    }
                }
            }
        }

        // println!("Filters checked PASS : {} in {} buckets", filters_checked, tokens_checked);


        return None;
    }
}

fn insert_dup<K, V>(map: &mut HashMap<K, Vec<V>>, k: K, v: V)
where
    K: std::cmp::Ord + std::hash::Hash,
{
    map.entry(k).or_insert_with(Vec::new).push(v)
}

fn remove_dup<K, V>(map: &mut HashMap<K, Vec<V>>, k: K)
where
    K: Ord + std::hash::Hash,
{
    if let Entry::Occupied(mut entry) = map.entry(k) {
        entry.get_mut().pop();
        if entry.get().is_empty() {
            entry.remove();
        }
    }
}

fn token_histogram<T>(filter_tokens: &Vec<(T, Vec<Vec<Hash>>)>) -> (u32, HashMap<Hash, u32>) {
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
mod parse_tests {
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
    #[ignore]
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
