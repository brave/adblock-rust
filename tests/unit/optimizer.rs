#[cfg(test)]
mod optimization_tests_pattern_group {
    #[cfg(test)]
    mod optimization_tests_pattern_group_tests {
        use super::*;
        use crate::lists;
        use crate::regex_manager::CompiledRegex;
        use crate::regex_manager::RegexManager;
        use crate::request::Request;
        use regex::bytes::RegexSetBuilder as BytesRegexSetBuilder;

        fn check_regex_match(regex: &CompiledRegex, pattern: &str, matches: bool) {
            let is_match = regex.is_match(pattern);
            assert!(
                is_match == matches,
                "Expected {regex} match {pattern} = {matches}"
            );
        }

        fn check_match(
            _regex_manager: &mut RegexManager,
            filter: &NetworkFilter,
            url_path: &str,
            matches: bool,
        ) {
            let is_match = filter.matches_test(
                &Request::new(
                    ("https://example.com/".to_string() + url_path).as_str(),
                    "https://google.com",
                    "",
                )
                .unwrap(),
            );
            assert!(
                is_match == matches,
                "Expected {filter} match {url_path} = {matches}"
            );
        }

        #[test]
        fn regex_set_works() {
            let regex_set = BytesRegexSetBuilder::new([
                r"/static/ad\.",
                "/static/ad-",
                "/static/ad/.*",
                "/static/ads/.*",
                "/static/adv/.*",
            ])
            .unicode(false)
            .build();

            let fused_regex = CompiledRegex::CompiledSet(regex_set.unwrap());
            assert!(matches!(fused_regex, CompiledRegex::CompiledSet(_)));
            check_regex_match(&fused_regex, "/static/ad.", true);
            check_regex_match(&fused_regex, "/static/ad-", true);
            check_regex_match(&fused_regex, "/static/ads-", false);
            check_regex_match(&fused_regex, "/static/ad/", true);
            check_regex_match(&fused_regex, "/static/ad", false);
            check_regex_match(&fused_regex, "/static/ad/foobar", true);
            check_regex_match(&fused_regex, "/static/ad/foobar/asd?q=1", true);
            check_regex_match(&fused_regex, "/static/ads/", true);
            check_regex_match(&fused_regex, "/static/ads", false);
            check_regex_match(&fused_regex, "/static/ads/foobar", true);
            check_regex_match(&fused_regex, "/static/ads/foobar/asd?q=1", true);
            check_regex_match(&fused_regex, "/static/adv/", true);
            check_regex_match(&fused_regex, "/static/adv", false);
            check_regex_match(&fused_regex, "/static/adv/foobar", true);
            check_regex_match(&fused_regex, "/static/adv/foobar/asd?q=1", true);
        }

        #[test]
        fn combines_simple_regex_patterns() {
            let rules = [
                "/static/ad-",
                "/static/ad.",
                "/static/ad/*",
                "/static/ads/*",
                "/static/adv/*",
            ];

            let (filters, _) = lists::parse_filters(rules, true, Default::default());

            let optimization = SimplePatternGroup {};

            filters
                .iter()
                .for_each(|f| assert!(optimization.select(f), "Expected rule to be selected"));

            let fused = optimization.fusion(&filters);

            assert!(!fused.is_regex(), "Expected rule to not be a regex");
            assert_eq!(
                fused.to_string(),
                "/static/ad- <+> /static/ad. <+> /static/ad/* <+> /static/ads/* <+> /static/adv/*"
            );
            let mut regex_manager = RegexManager::default();
            check_match(&mut regex_manager, &fused, "/static/ad-", true);
            check_match(&mut regex_manager, &fused, "/static/ad.", true);
            check_match(&mut regex_manager, &fused, "/static/ad%", false);
            check_match(&mut regex_manager, &fused, "/static/ads-", false);
            check_match(&mut regex_manager, &fused, "/static/ad/", true);
            check_match(&mut regex_manager, &fused, "/static/ad", false);
            check_match(&mut regex_manager, &fused, "/static/ad/foobar", true);
            check_match(
                &mut regex_manager,
                &fused,
                "/static/ad/foobar/asd?q=1",
                true,
            );
            check_match(&mut regex_manager, &fused, "/static/ads/", true);
            check_match(&mut regex_manager, &fused, "/static/ads", false);
            check_match(&mut regex_manager, &fused, "/static/ads/foobar", true);
            check_match(
                &mut regex_manager,
                &fused,
                "/static/ads/foobar/asd?q=1",
                true,
            );
            check_match(&mut regex_manager, &fused, "/static/adv/", true);
            check_match(&mut regex_manager, &fused, "/static/adv", false);
            check_match(&mut regex_manager, &fused, "/static/adv/foobar", true);
            check_match(
                &mut regex_manager,
                &fused,
                "/static/adv/foobar/asd?q=1",
                true,
            );
        }

        #[test]
        fn separates_pattern_by_grouping() {
            let rules = [
                "/analytics-v1.",
                "/v1/pixel?",
                "/api/v1/stat?",
                "/analytics/v1/*$domain=~my.leadpages.net",
                "/v1/ads/*",
            ];

            let (filters, _) = lists::parse_filters(rules, true, Default::default());

            let optimization = SimplePatternGroup {};

            let (fused, skipped) = apply_optimisation(&optimization, filters);

            assert_eq!(fused.len(), 1);
            let filter = fused.first().unwrap();
            assert_eq!(
                filter.to_string(),
                "/analytics-v1. <+> /v1/pixel? <+> /api/v1/stat? <+> /v1/ads/*"
            );

            assert!(filter.matches_test(
                &Request::new(
                    "https://example.com/v1/pixel?",
                    "https://my.leadpages.net",
                    ""
                )
                .unwrap()
            ));

            assert_eq!(skipped.len(), 1);
            let filter = skipped.first().unwrap();
            assert_eq!(
                filter.to_string(),
                "/analytics/v1/*$domain=~my.leadpages.net"
            );

            assert!(filter.matches_test(
                &Request::new(
                    "https://example.com/analytics/v1/foobar",
                    "https://foo.leadpages.net",
                    ""
                )
                .unwrap()
            ))
        }
    }

    /*
    #[cfg(test)]
    mod optimization_tests_union_domain {
        use super::*;
        use crate::filters::network::NetworkMatchable;
        use crate::lists;
        use crate::request::Request;
        use crate::utils;

        #[test]
        fn merges_domains() {
            let rules = [
                "/analytics-v1$domain=google.com",
                "/analytics-v1$domain=example.com",
            ];

            let (filters, _) = lists::parse_filters(&rules, true, Default::default());
            let optimization = UnionDomainGroup {};
            let (fused, _) = apply_optimisation(&optimization, filters);

            assert_eq!(fused.len(), 1);
            let filter = fused.get(0).unwrap();
            assert_eq!(
                filter.to_string(),
                "/analytics-v1$domain=google.com <+> /analytics-v1$domain=example.com"
            );

            let expected_domains = vec![
                utils::fast_hash("example.com"),
                utils::fast_hash("google.com"),
            ];
            assert!(filter.opt_domains.is_some());
            let filter_domains = filter.opt_domains.as_ref().unwrap();
            for dom in expected_domains {
                assert!(filter_domains.contains(&dom));
            }

            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://google.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://foo.leadpages.net",
                        ""
                    )
                    .unwrap()
                ) == false
            );
        }

        #[test]
        fn skips_rules_with_no_domain() {
            let rules = [
                "/analytics-v1$domain=google.com",
                "/analytics-v1$domain=example.com",
                "/analytics-v1",
            ];

            let (filters, _) = lists::parse_filters(&rules, true, Default::default());
            let optimization = UnionDomainGroup {};
            let (_, skipped) = apply_optimisation(&optimization, filters);

            assert_eq!(skipped.len(), 1);
            let filter = skipped.get(0).unwrap();
            assert_eq!(filter.to_string(), "/analytics-v1");
        }

        #[test]
        fn optimises_domains() {
            let rules = [
                "/analytics-v1$domain=google.com",
                "/analytics-v1$domain=example.com",
                "/analytics-v1$domain=exampleone.com|exampletwo.com",
                "/analytics-v1",
            ];

            let (filters, _) = lists::parse_filters(&rules, true, Default::default());

            let optimization = UnionDomainGroup {};

            let (fused, skipped) = apply_optimisation(&optimization, filters);

            assert_eq!(fused.len(), 1);
            let filter = fused.get(0).unwrap();
            assert_eq!(
                filter.to_string(),
                "/analytics-v1$domain=google.com <+> /analytics-v1$domain=example.com <+> /analytics-v1$domain=exampleone.com|exampletwo.com"
            );

            assert_eq!(skipped.len(), 1);
            let skipped_filter = skipped.get(0).unwrap();
            assert_eq!(skipped_filter.to_string(), "/analytics-v1");

            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://google.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://example.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://exampletwo.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
            assert!(
                filter.matches_test(
                    &Request::new(
                        "https://example.com/analytics-v1/foobar",
                        "https://foo.leadpages.net",
                        ""
                    )
                    .unwrap()
                ) == false
            );
        }
    }
    */
    use super::super::*;
    use crate::lists;
    use crate::regex_manager::CompiledRegex;
    use crate::regex_manager::RegexManager;
    use crate::request::Request;
    use regex::bytes::RegexSetBuilder as BytesRegexSetBuilder;

    fn check_regex_match(regex: &CompiledRegex, pattern: &str, matches: bool) {
        let is_match = regex.is_match(pattern);
        assert!(
            is_match == matches,
            "Expected {regex} match {pattern} = {matches}"
        );
    }

    fn check_match(
        _regex_manager: &mut RegexManager,
        filter: &NetworkFilter,
        url_path: &str,
        matches: bool,
    ) {
        let is_match = filter.matches_test(
            &Request::new(
                ("https://example.com/".to_string() + url_path).as_str(),
                "https://google.com",
                "",
            )
            .unwrap(),
        );
        assert!(
            is_match == matches,
            "Expected {filter} match {url_path} = {matches}"
        );
    }

    #[test]
    fn regex_set_works() {
        let regex_set = BytesRegexSetBuilder::new([
            r"/static/ad\.",
            "/static/ad-",
            "/static/ad/.*",
            "/static/ads/.*",
            "/static/adv/.*",
        ])
        .unicode(false)
        .build();

        let fused_regex = CompiledRegex::CompiledSet(regex_set.unwrap());
        assert!(matches!(fused_regex, CompiledRegex::CompiledSet(_)));
        check_regex_match(&fused_regex, "/static/ad.", true);
        check_regex_match(&fused_regex, "/static/ad-", true);
        check_regex_match(&fused_regex, "/static/ads-", false);
        check_regex_match(&fused_regex, "/static/ad/", true);
        check_regex_match(&fused_regex, "/static/ad", false);
        check_regex_match(&fused_regex, "/static/ad/foobar", true);
        check_regex_match(&fused_regex, "/static/ad/foobar/asd?q=1", true);
        check_regex_match(&fused_regex, "/static/ads/", true);
        check_regex_match(&fused_regex, "/static/ads", false);
        check_regex_match(&fused_regex, "/static/ads/foobar", true);
        check_regex_match(&fused_regex, "/static/ads/foobar/asd?q=1", true);
        check_regex_match(&fused_regex, "/static/adv/", true);
        check_regex_match(&fused_regex, "/static/adv", false);
        check_regex_match(&fused_regex, "/static/adv/foobar", true);
        check_regex_match(&fused_regex, "/static/adv/foobar/asd?q=1", true);
    }

    #[test]
    fn combines_simple_regex_patterns() {
        let rules = [
            "/static/ad-",
            "/static/ad.",
            "/static/ad/*",
            "/static/ads/*",
            "/static/adv/*",
        ];

        let (filters, _) = lists::parse_filters(rules, true, Default::default());

        let optimization = SimplePatternGroup {};

        filters
            .iter()
            .for_each(|f| assert!(optimization.select(f), "Expected rule to be selected"));

        let fused = optimization.fusion(&filters);

        assert!(!fused.is_regex(), "Expected rule to not be a regex");
        assert_eq!(
            fused.to_string(),
            "/static/ad- <+> /static/ad. <+> /static/ad/* <+> /static/ads/* <+> /static/adv/*"
        );
        let mut regex_manager = RegexManager::default();
        check_match(&mut regex_manager, &fused, "/static/ad-", true);
        check_match(&mut regex_manager, &fused, "/static/ad.", true);
        check_match(&mut regex_manager, &fused, "/static/ad%", false);
        check_match(&mut regex_manager, &fused, "/static/ads-", false);
        check_match(&mut regex_manager, &fused, "/static/ad/", true);
        check_match(&mut regex_manager, &fused, "/static/ad", false);
        check_match(&mut regex_manager, &fused, "/static/ad/foobar", true);
        check_match(
            &mut regex_manager,
            &fused,
            "/static/ad/foobar/asd?q=1",
            true,
        );
        check_match(&mut regex_manager, &fused, "/static/ads/", true);
        check_match(&mut regex_manager, &fused, "/static/ads", false);
        check_match(&mut regex_manager, &fused, "/static/ads/foobar", true);
        check_match(
            &mut regex_manager,
            &fused,
            "/static/ads/foobar/asd?q=1",
            true,
        );
        check_match(&mut regex_manager, &fused, "/static/adv/", true);
        check_match(&mut regex_manager, &fused, "/static/adv", false);
        check_match(&mut regex_manager, &fused, "/static/adv/foobar", true);
        check_match(
            &mut regex_manager,
            &fused,
            "/static/adv/foobar/asd?q=1",
            true,
        );
    }

    #[test]
    fn separates_pattern_by_grouping() {
        let rules = [
            "/analytics-v1.",
            "/v1/pixel?",
            "/api/v1/stat?",
            "/analytics/v1/*$domain=~my.leadpages.net",
            "/v1/ads/*",
        ];

        let (filters, _) = lists::parse_filters(rules, true, Default::default());

        let optimization = SimplePatternGroup {};

        let (fused, skipped) = apply_optimisation(&optimization, filters);

        assert_eq!(fused.len(), 1);
        let filter = fused.first().unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics-v1. <+> /v1/pixel? <+> /api/v1/stat? <+> /v1/ads/*"
        );

        assert!(filter.matches_test(
            &Request::new(
                "https://example.com/v1/pixel?",
                "https://my.leadpages.net",
                ""
            )
            .unwrap()
        ));

        assert_eq!(skipped.len(), 1);
        let filter = skipped.first().unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics/v1/*$domain=~my.leadpages.net"
        );

        assert!(filter.matches_test(
            &Request::new(
                "https://example.com/analytics/v1/foobar",
                "https://foo.leadpages.net",
                ""
            )
            .unwrap()
        ))
    }
}

/*
#[cfg(test)]
mod optimization_tests_union_domain {
    use super::*;
    use crate::filters::network::NetworkMatchable;
    use crate::lists;
    use crate::request::Request;
    use crate::utils;

    #[test]
    fn merges_domains() {
        let rules = [
            "/analytics-v1$domain=google.com",
            "/analytics-v1$domain=example.com",
        ];

        let (filters, _) = lists::parse_filters(&rules, true, Default::default());
        let optimization = UnionDomainGroup {};
        let (fused, _) = apply_optimisation(&optimization, filters);

        assert_eq!(fused.len(), 1);
        let filter = fused.get(0).unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics-v1$domain=google.com <+> /analytics-v1$domain=example.com"
        );

        let expected_domains = vec![
            utils::fast_hash("example.com"),
            utils::fast_hash("google.com"),
        ];
        assert!(filter.opt_domains.is_some());
        let filter_domains = filter.opt_domains.as_ref().unwrap();
        for dom in expected_domains {
            assert!(filter_domains.contains(&dom));
        }

        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://google.com",
                    ""
                )
                .unwrap()
            ) == true
        );
        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://foo.leadpages.net",
                    ""
                )
                .unwrap()
            ) == false
        );
    }

    #[test]
    fn skips_rules_with_no_domain() {
        let rules = [
            "/analytics-v1$domain=google.com",
            "/analytics-v1$domain=example.com",
            "/analytics-v1",
        ];

        let (filters, _) = lists::parse_filters(&rules, true, Default::default());
        let optimization = UnionDomainGroup {};
        let (_, skipped) = apply_optimisation(&optimization, filters);

        assert_eq!(skipped.len(), 1);
        let filter = skipped.get(0).unwrap();
        assert_eq!(filter.to_string(), "/analytics-v1");
    }

    #[test]
    fn optimises_domains() {
        let rules = [
            "/analytics-v1$domain=google.com",
            "/analytics-v1$domain=example.com",
            "/analytics-v1$domain=exampleone.com|exampletwo.com",
            "/analytics-v1",
        ];

        let (filters, _) = lists::parse_filters(&rules, true, Default::default());

        let optimization = UnionDomainGroup {};

        let (fused, skipped) = apply_optimisation(&optimization, filters);

        assert_eq!(fused.len(), 1);
        let filter = fused.get(0).unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics-v1$domain=google.com <+> /analytics-v1$domain=example.com <+> /analytics-v1$domain=exampleone.com|exampletwo.com"
        );

        assert_eq!(skipped.len(), 1);
        let skipped_filter = skipped.get(0).unwrap();
        assert_eq!(skipped_filter.to_string(), "/analytics-v1");

        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://google.com",
                    ""
                )
                .unwrap()
            ) == true
        );
        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://example.com",
                    ""
                )
                .unwrap()
            ) == true
        );
        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://exampletwo.com",
                    ""
                )
                .unwrap()
            ) == true
        );
        assert!(
            filter.matches_test(
                &Request::new(
                    "https://example.com/analytics-v1/foobar",
                    "https://foo.leadpages.net",
                    ""
                )
                .unwrap()
            ) == false
        );
    }
}
*/
