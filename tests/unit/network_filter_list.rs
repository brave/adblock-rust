#[cfg(test)]
mod tests {
    use crate::filters::network::NetworkFilter;
    use crate::network_filter_list::{
        insert_dup, token_histogram, NetworkFilterList, NetworkFilterListTrait,
    };
    use crate::regex_manager::RegexManager;
    use crate::request::Request;
    use crate::utils::{fast_hash, Hash};
    use std::collections::{HashMap, HashSet};

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
            Some(&vec![String::from("bar"), String::from("foo")])
        );

        // inserts into another key item
        insert_dup(&mut dup_map, 123, String::from("baz"));
        assert_eq!(dup_map.get(&123), Some(&vec![String::from("baz")]));
        assert_eq!(
            dup_map.get(&1),
            Some(&vec![String::from("bar"), String::from("foo")])
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
            let filters = ["||foo.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true, Default::default()))
                .filter_map(Result::ok)
                .collect();
            let filter_list = NetworkFilterList::new(network_filters, false);
            let maybe_matching_filter = filter_list.filter_map.get(&fast_hash("foo"));
            assert!(maybe_matching_filter.is_some(), "Expected filter not found");
        }
        // choses least frequent token
        {
            let filters = ["||foo.com", "||bar.com/foo"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true, Default::default()))
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
            let filters = ["||foo.com", "||foo.com/bar", "||www"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true, Default::default()))
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
            let filters = ["||foo.com", "||foo.com$domain=bar.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true, Default::default()))
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
            let filters = ["foo*$domain=bar.com|baz.com"];
            let network_filters: Vec<_> = filters
                .into_iter()
                .map(|f| NetworkFilter::parse(&f, true, Default::default()))
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

    fn test_requests_filters(
        filters: impl IntoIterator<Item = impl AsRef<str>>,
        requests: &[(Request, bool)],
    ) {
        let network_filters: Vec<_> = filters
            .into_iter()
            .map(|f| NetworkFilter::parse(&f.as_ref(), true, Default::default()))
            .filter_map(Result::ok)
            .collect();
        let filter_list = NetworkFilterList::new(network_filters, false);
        let mut regex_manager = RegexManager::default();

        requests.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = filter_list.check(&req, &HashSet::new(), &mut regex_manager);
            if *expected_result {
                assert!(matched_rule.is_some(), "Expected match for {}", req.url);
            } else {
                assert!(
                    matched_rule.is_none(),
                    "Expected no match for {}, matched with {}",
                    req.url,
                    matched_rule.unwrap().to_string()
                );
            }
        });
    }

    #[test]
    fn network_filter_list_check_works_plain_filter() {
        // includes cases with fall back to 0 bucket (no tokens from a rule)
        let filters = [
            "foo",
            "-foo-",
            "&fo.o=+_-",
            "foo/bar/baz",
            "com/bar/baz",
            "https://bar.com/bar/baz",
        ];

        let url_results = [
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
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_hostname_anchor() {
        let filters = [
            "||foo.com",
            "||bar.com/bar",
            "||coo.baz.",
            "||foo.bar.com^",
            "||foo.baz^",
        ];

        let url_results = [
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
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_unicode() {
        let filters = [
            "||firstrowsports.li/frame/",
            "||fırstrowsports.eu/pu/",
            "||atđhe.net/pu/",
        ];

        let url_results = [
            ("https://firstrowsports.li/frame/bar", true),
            ("https://secondrowsports.li/frame/bar", false),
            ("https://fırstrowsports.eu/pu/foo", true),
            ("https://xn--frstrowsports-39b.eu/pu/foo", true),
            ("https://atđhe.net/pu/foo", true),
            ("https://xn--athe-1ua.net/pu/foo", true),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        test_requests_filters(&filters, &request_expectations);
    }

    #[test]
    fn network_filter_list_check_works_regex_escaping() {
        let filters = [
            r#"/^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/$domain=123movies.com|1337x.to"#,
            r#"/\:\/\/data.*\.com\/[a-zA-Z0-9]{30,}/$third-party,xmlhttprequest"#,
        ];

        let url_results = [
            (
                Request::new("https://bit.ly/bar/", "http://123movies.com", "").unwrap(),
                true,
            ),
            (
                Request::new(
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
