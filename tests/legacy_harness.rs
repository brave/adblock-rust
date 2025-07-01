mod legacy_test_filters {
    use adblock::filters::network::NetworkFilter;
    use adblock::filters::network::NetworkFilterMask;
    use adblock::filters::network::NetworkMatchable;
    use adblock::regex_manager::RegexManager;
    use adblock::request::Request;

    fn test_filter<'a>(
        raw_filter: &str,
        expected_filter_mask: NetworkFilterMask,
        expected_filter: Option<&'a str>,
        blocked: &[&'a str],
        not_blocked: &[&'a str],
    ) {
        let filter_res = NetworkFilter::parse(raw_filter, true, Default::default());
        assert!(
            filter_res.is_ok(),
            "Parsing {} failed: {:?}",
            raw_filter,
            filter_res.err()
        );
        let filter = filter_res.unwrap();

        assert_eq!(
            filter.mask, expected_filter_mask,
            "Filter {} mask doesn't match expectation",
            raw_filter
        );

        let filter_string = filter.filter.string_view();
        let filter_part = filter_string.as_deref();
        assert!(
            expected_filter == filter_part,
            "Expected filter to be {:?}, found {:?}",
            expected_filter,
            filter.filter
        );

        for to_block in blocked {
            assert!(
                filter.matches(
                    &Request::new(to_block, "https://example.com", "other").unwrap(),
                    &mut RegexManager::default()
                ),
                "Expected filter {} to match {}",
                raw_filter,
                &to_block
            );
        }

        for to_pass in not_blocked {
            assert!(
                !filter.matches(
                    &Request::new(to_pass, "https://example.com", "other").unwrap(),
                    &mut RegexManager::default()
                ),
                "Expected filter {} to pass {}",
                raw_filter,
                &to_pass
            );
        }
    }

    #[test]
    fn check_default_wildcard() {
        test_filter(
            "/banner/*/img",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("/banner/*/img"),
            &[
                "http://example.com/banner/foo/img",
                "http://example.com/banner/foo/bar/img?param",
                "http://example.com/banner//img/foo",
                "http://example.com/banner//img.gif",
            ],
            &[
                "http://example.com/banner",
                "http://example.com/banner/",
                "http://example.com/banner/img",
                "http://example.com/img/banner/",
            ],
        );
    }

    #[test]
    fn check_default_separator() {
        test_filter(
            "/banner/*/img^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("/banner/*/img^"),
            &[
                "http://example.com/banner/foo/img",
                "http://example.com/banner/foo/bar/img?param",
                "http://example.com/banner//img/foo",
            ],
            &[
                "http://example.com/banner/img",
                "http://example.com/banner/foo/imgraph",
                "http://example.com/banner/foo/img.gif",
            ],
        );
    }

    #[test]
    fn check_hostname_right_anchor() {
        test_filter(
            "||ads.example.com^",
            NetworkFilterMask::DEFAULT_OPTIONS
                | NetworkFilterMask::FROM_DOCUMENT
                | NetworkFilterMask::IS_RIGHT_ANCHOR
                | NetworkFilterMask::IS_HOSTNAME_ANCHOR, // FTHostAnchored | FTHostOnly
            None,
            &[
                "http://ads.example.com/foo.gif",
                "http://server1.ads.example.com/foo.gif",
                "https://ads.example.com:8000/",
            ],
            &[
                "http://ads.example.com.ua/foo.gif",
                "http://example.com/redirect/http://ads.example.com/",
            ],
        );
    }

    #[test]
    fn check_left_right_anchor() {
        test_filter(
            "|http://example.com/|",
            NetworkFilterMask::DEFAULT_OPTIONS
                | NetworkFilterMask::IS_LEFT_ANCHOR
                | NetworkFilterMask::IS_RIGHT_ANCHOR, // FTLeftAnchored | FTRightAnchored
            Some("http://example.com/"),
            &["http://example.com/"],
            &[
                "http://example.com/foo.gif",
                "http://example.info/redirect/http://example.com/",
            ],
        );
    }

    #[test]
    fn check_right_anchor() {
        test_filter(
            "swf|",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_RIGHT_ANCHOR,
            Some("swf"),
            &["http://example.com/annoyingflash.swf"],
            &["http://example.com/swf/index.html"],
        );
    }

    #[test]
    fn check_left_anchor() {
        test_filter(
            "|http://baddomain.example/",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_LEFT_ANCHOR,
            Some("http://baddomain.example/"),
            &["http://baddomain.example/banner.gif"],
            &["http://gooddomain.example/analyze?http://baddomain.example"],
        );
    }

    #[test]
    fn check_hostname_anchor() {
        test_filter(
            "||example.com/banner.gif",
            NetworkFilterMask::DEFAULT_OPTIONS
            | NetworkFilterMask::IS_LEFT_ANCHOR      // filter part of the rule is left-anchored (to hostname)
            | NetworkFilterMask::IS_HOSTNAME_ANCHOR, // FTHostAnchored, FONoFilterOption
            Some("/banner.gif"),
            &[
                "http://example.com/banner.gif",
                "https://example.com/banner.gif",
                "http://www.example.com/banner.gif",
            ],
            &[
                "http://badexample.com/banner.gif",
                "http://gooddomain.example/analyze?http://example.com/banner.gif",
                "http://example.com.au/banner.gif",
                "http://example.com/banner2.gif",
            ],
        );
    }

    #[test]
    fn check_match_port() {
        test_filter(
            "http://example.com^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("http://example.com^"),
            &["http://example.com/", "http://example.com:8000/ "],
            &[],
        );
    }

    #[test]
    fn check_hostlike_separators() {
        test_filter(
            "^example.com^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("^example.com^"),
            &["http://example.com:8000/foo.bar?a=12&b=%D1%82%D0%B5%D1%81%D1%82"],
            &[],
        );
    }

    #[test]
    fn check_escaped() {
        test_filter(
            "^%D1%82%D0%B5%D1%81%D1%82^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some(&"^%D1%82%D0%B5%D1%81%D1%82^".to_lowercase()),
            &["http://example.com:8000/foo.bar?a=12&b=%D1%82%D0%B5%D1%81%D1%82"],
            &["http://example.com:8000/foo.bar?a=12&b%D1%82%D0%B5%D1%81%D1%823"],
        );
    }

    #[test]
    fn check_separators() {
        test_filter(
            "^foo.bar^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("^foo.bar^"),
            &["http://example.com:8000/foo.bar?a=12&b=%D1%82%D0%B5%D1%81%D1%82"],
            &[],
        );
    }
    #[test]
    fn check_separators_simple() {
        test_filter(
            "^promotion^",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_REGEX,
            Some("^promotion^"),
            &["http://test.com/promotion/test"],
            &[],
        );
    }

    #[test]
    fn check_full_regex() {
        test_filter(
            "/banner[0-9]+/",
            NetworkFilterMask::DEFAULT_OPTIONS | NetworkFilterMask::IS_COMPLETE_REGEX,
            Some("/banner[0-9]+/"),
            &[
                "http://example.com/banner123",
                "http://example.com/testbanner1",
            ],
            &[
                "http://example.com/banners",
                "http://example.com/banners123",
            ],
        );
    }

    #[test]
    fn check_hostname_exact_match() {
        test_filter(
            "||static.tumblr.com/dhqhfum/WgAn39721/cfh_header_banner_v2.jpg",
            NetworkFilterMask::DEFAULT_OPTIONS
            | NetworkFilterMask::IS_LEFT_ANCHOR      // filter part left-anchored to hostname
            | NetworkFilterMask::IS_HOSTNAME_ANCHOR, // FTHostAnchored, FONoFilterOption
            Some(&"/dhqhfum/WgAn39721/cfh_header_banner_v2.jpg".to_lowercase()), // by default rules are case-insensitive, everything gets lowercased
            &["http://static.tumblr.com/dhqhfum/WgAn39721/cfh_header_banner_v2.jpg"],
            &[],
        );
    }

    #[test]
    fn check_third_party() {
        test_filter(
            "||googlesyndication.com/safeframe/$third-party",
            NetworkFilterMask::FROM_NETWORK_TYPES
            | NetworkFilterMask::FROM_HTTP
            | NetworkFilterMask::FROM_HTTPS
            | NetworkFilterMask::THIRD_PARTY
            | NetworkFilterMask::IS_LEFT_ANCHOR      // filter part left-anchored to hostname
            | NetworkFilterMask::IS_HOSTNAME_ANCHOR, // FTHostAnchored, FOThirdParty
            Some("/safeframe/"),
            &[concat!(
                "http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html",
                r"#xpc=sf-gdn-exp-2&p=http%3A//slashdot.org;"
            )],
            &[],
        );
    }
    #[test]
    fn check_third_party_script() {
        test_filter(
            "||googlesyndication.com/safeframe/$third-party,script",
            NetworkFilterMask::FROM_SCRIPT
            | NetworkFilterMask::FROM_HTTP
            | NetworkFilterMask::FROM_HTTPS
            | NetworkFilterMask::THIRD_PARTY
            | NetworkFilterMask::IS_LEFT_ANCHOR      // filter part left-anchored to hostname
            | NetworkFilterMask::IS_HOSTNAME_ANCHOR, // FTHostAnchored, FOThirdParty, FOScript
            Some("/safeframe/"),
            &[
                // handle the sample below to avoid hacking code around just to pass the request that matches script option
                // "http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html#xpc=sf-gdn-exp-2&p=http%3A//slashdot.org;"
            ],
            &[],
        );

        // explicit, separate testcase construction of the "script" option as it is not the deafult
        let filter = NetworkFilter::parse(
            "||googlesyndication.com/safeframe/$third-party,script",
            true,
            Default::default(),
        )
        .unwrap();
        let request = Request::new("http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html#xpc=sf-gdn-exp-2&p=http%3A//slashdot.org;", "https://this-is-always-third-party.com", "script").unwrap();
        assert!(filter.matches(&request, &mut RegexManager::default()));
    }
}

mod legacy_check_match {
    use adblock::request::Request;
    use adblock::Engine;

    fn check_match<'a>(
        rules: &[&'a str],
        blocked: &[&'a str],
        not_blocked: &[&'a str],
        tags: &[&'a str],
    ) {
        let mut engine = Engine::from_rules(rules, Default::default()); // first one with the provided rules
        engine.use_tags(tags);

        let mut engine_deserialized = Engine::default(); // second empty
        engine_deserialized.use_tags(tags);
        {
            let engine_serialized = engine.serialize().unwrap();
            engine_deserialized.deserialize(&engine_serialized).unwrap(); // override from serialized copy
        }

        for to_block in blocked {
            let request = Request::new(to_block, "alwaysthirdparty.com", "script").unwrap();

            assert!(
                engine.check_network_request(&request).matched,
                "Expected engine from {:?} to match {}",
                rules,
                &to_block
            );

            assert!(
                engine_deserialized.check_network_request(&request).matched,
                "Expected deserialized engine from {:?} to match {}",
                rules,
                &to_block
            );
        }

        for to_pass in not_blocked {
            let request = Request::new(to_pass, "alwaysthirdparty.com", "script").unwrap();

            assert!(
                !engine.check_network_request(&request).matched,
                "Expected engine from {:?} to not match {}",
                rules,
                &to_pass
            );

            assert!(
                !engine_deserialized.check_network_request(&request).matched,
                "Expected deserialized engine from {:?} to not match {}",
                rules,
                &to_pass
            );
        }
    }

    #[test]
    fn exception_rules() {
        check_match(
            &["adv", "@@advice."],
            &["http://example.com/advert.html"],
            &["http://example.com/advice.html"],
            &[],
        );

        check_match(
            &["@@|http://example.com", "@@advice.", "adv", "!foo"],
            &["http://examples.com/advert.html"],
            &[
                "http://example.com/advice.html",
                "http://example.com/advert.html",
                "http://examples.com/advice.html",
                "http://examples.com/#!foo",
            ],
            &[],
        );

        {
            // Explicitly write out the full case instead of using check_match helper
            // or tweaking it to allow passing in the source domain for this one case
            let engine = Engine::from_rules(
                [
                    "/ads/freewheel/*",
                    "@@||turner.com^*/ads/freewheel/*/AdManager.js$domain=cnn.com",
                ],
                Default::default(),
            );
            let mut engine_deserialized = Engine::default(); // second empty
            {
                let engine_serialized = engine.serialize().unwrap();
                engine_deserialized.deserialize(&engine_serialized).unwrap(); // override from serialized copy
            }

            let request = Request::new(
                "http://z.cdn.turner.com/xslo/cvp/ads/freewheel/js/0/AdManager.js",
                "http://cnn.com",
                "",
            )
            .unwrap();
            assert!(!engine.check_network_request(&request).matched);
            assert!(!engine_deserialized.check_network_request(&request).matched);
        }

        check_match(
            &["^promotion^"],
            &["http://yahoo.co.jp/promotion/imgs"],
            &[],
            &[],
        );

        check_match(
            &["^ads^"],
            &[
                "http://yahoo.co.jp/ads/imgs",
                "http://yahoo.co.jp/ads",
                "http://yahoo.co.jp/ads?xyz",
                "http://yahoo.co.jp/xyz?ads",
            ],
            &[
                "http://yahoo.co.jp/uploads/imgs",
                "http://yahoo.co.jp/adsx/imgs",
                "http://yahoo.co.jp/adsshmads/imgs",
                "ads://ads.co.ads/aads",
            ],
            &[],
        );
    }

    #[test]
    fn tag_tests() {
        // No matching tags should not match a tagged filter
        check_match(
            &[
                "adv$tag=stuff",
                "somelongpath/test$tag=stuff",
                "||brianbondy.com/$tag=brian",
                "||brave.com$tag=brian",
            ],
            &[],
            &[
                "http://example.com/advert.html",
                "http://example.com/somelongpath/test/2.html",
                "https://brianbondy.com/about",
                "https://brave.com/about",
            ],
            &[],
        );
        // A matching tag should match a tagged filter
        check_match(
            &[
                "adv$tag=stuff",
                "somelongpath/test$tag=stuff",
                "||brianbondy.com/$tag=brian",
                "||brave.com$tag=brian",
            ],
            &[
                "http://example.com/advert.html",
                "http://example.com/somelongpath/test/2.html",
                "https://brianbondy.com/about",
                "https://brave.com/about",
            ],
            &[],
            &["stuff", "brian"],
        );

        // A tag which doesn't match shouldn't match
        check_match(
            &[
                "adv$tag=stuff",
                "somelongpath/test$tag=stuff",
                "||brianbondy.com/$tag=brian",
                "||brave.com$tag=brian",
            ],
            &[],
            &[
                "http://example.com/advert.html",
                "http://example.com/somelongpath/test/2.html",
                "https://brianbondy.com/about",
                "https://brave.com/about",
            ],
            &["filtertag1", "filtertag2"],
        );
    }
}

mod legacy_check_options {
    use adblock::request::Request;
    use adblock::Engine;

    fn check_option_rule<'a>(rules: &[&'a str], tests: &[(&'a str, &'a str, &'a str, bool)]) {
        let engine = Engine::from_rules(rules, Default::default()); // first one with the provided rules

        for (url, source_url, request_type, expectation) in tests {
            let request = Request::new(url, source_url, request_type).unwrap();
            assert!(
                engine.check_network_request(&request).matched == *expectation,
                "Expected match = {} for {} from {} typed {} against {:?}",
                expectation,
                url,
                source_url,
                request_type,
                rules
            )
        }
    }

    #[test]
    fn option_no_option() {
        check_option_rule(
            &["||example.com"],
            &[
                ("http://example.com", "https://example.com", "", true),
                ("http://example2.com", "https://example.com", "", false),
                ("http://example.com", "https://example.com", "", true),
            ],
        );
    }

    #[test]
    fn check_options_third_party() {
        check_option_rule(
            &["||example.com^$third-party"],
            &[
                (
                    "http://example.com",
                    "http://brianbondy.com",
                    "script",
                    true,
                ),
                ("http://example.com", "http://example.com", "script", false),
                (
                    "http://ad.example.com",
                    "http://brianbondy.com",
                    "script",
                    true,
                ),
                (
                    "http://ad.example.com",
                    "http://example.com",
                    "script",
                    false,
                ),
                (
                    "http://example2.com",
                    "http://brianbondy.com",
                    "script",
                    false,
                ),
                ("http://example2.com", "http://example.com", "script", false),
                (
                    "http://example.com.au",
                    "http://brianbondy.com",
                    "script",
                    false,
                ),
                (
                    "http://example.com.au",
                    "http://example.com",
                    "script",
                    false,
                ),
            ],
        );
    }

    #[test]
    fn check_options_ping() {
        // We should block ping rules if the resource type is FOPing
        check_option_rule(
            &["||example.com^$ping"],
            &[
                ("http://example.com", "http://example.com", "ping", true),
                ("http://example.com", "http://example.com", "image", false),
            ],
        );
    }

    #[test]
    fn check_options_popup() {
        // Make sure we ignore popup rules for now
        check_option_rule(
            &["||example.com^$popup"],
            &[("http://example.com", "http://example.com", "popup", false)],
        );
    }

    #[test]
    fn check_options_third_party_notscript() {
        check_option_rule(
            &["||example.com^$third-party,~script"],
            &[
                ("http://example.com", "http://example2.com", "script", false),
                ("http://example.com", "http://example2.com", "other", true),
                ("http://example2.com", "http://example2.com", "other", false),
                ("http://example.com", "http://example.com", "other", false),
            ],
        );
    }

    #[test]
    fn check_options_domain_list() {
        check_option_rule(
            &["adv$domain=example.com|example.net"],
            &[
                ("http://example.net/adv", "http://example.com", "", true),
                ("http://somewebsite.com/adv", "http://example.com", "", true),
                (
                    "http://www.example.net/adv",
                    "http://www.example.net",
                    "",
                    true,
                ),
                (
                    "http://my.subdomain.example.com/adv",
                    "http://my.subdomain.example.com",
                    "",
                    true,
                ),
                (
                    "http://my.subdomain.example.com/adv",
                    "http://my.subdomain.example.com",
                    "",
                    true,
                ),
                ("http://example.com/adv", "http://badexample.com", "", false),
                (
                    "http://example.com/adv",
                    "http://otherdomain.net",
                    "",
                    false,
                ),
                ("http://example.net/ad", "http://example.com", "", false),
            ],
        );

        check_option_rule(
            &["adv$domain=~example.com"],
            &[
                ("http://example.net/adv", "http://otherdomain.com", "", true),
                (
                    "http://somewebsite.com/adv",
                    "http://example.com",
                    "",
                    false,
                ),
            ],
        );

        check_option_rule(
            &["adv$domain=~example.com|~example.net"],
            &[
                ("http://example.net/adv", "http://example.net", "", false),
                (
                    "http://somewebsite.com/adv",
                    "http://example.com",
                    "",
                    false,
                ),
                (
                    "http://www.example.net/adv",
                    "http://www.example.net",
                    "",
                    false,
                ),
                (
                    "http://my.subdomain.example.com/adv",
                    "http://my.subdomain.example.com",
                    "",
                    false,
                ),
                ("http://example.com/adv", "http://badexample.com", "", true),
                ("http://example.com/adv", "http://otherdomain.net", "", true),
                ("http://example.net/ad", "http://example.net", "", false),
            ],
        );

        check_option_rule(
            &["adv$domain=example.com|~example.net"],
            &[
                ("http://example.net/adv", "http://example.net", "", false),
                ("http://somewebsite.com/adv", "http://example.com", "", true),
                (
                    "http://www.example.net/adv",
                    "http://www.example.net",
                    "",
                    false,
                ),
                (
                    "http://my.subdomain.example.com/adv",
                    "http://my.subdomain.example.com",
                    "",
                    true,
                ),
                ("http://example.com/adv", "http://badexample.com", "", false),
                (
                    "http://example.com/adv",
                    "http://otherdomain.net",
                    "",
                    false,
                ),
                ("http://example.net/ad", "http://example.net", "", false),
            ],
        );
    }

    #[test]
    fn check_options_domain_not_subdomain() {
        check_option_rule(
            &["adv$domain=example.com|~foo.example.com"],
            &[
                ("http://example.net/adv", "http://example.com", "", true),
                (
                    "http://example.net/adv",
                    "http://foo.example.com",
                    "",
                    false,
                ),
                (
                    "http://example.net/adv",
                    "http://www.foo.example.com",
                    "",
                    false,
                ),
            ],
        );

        check_option_rule(
            &["adv$domain=~example.com|foo.example.com"],
            &[
                ("http://example.net/adv", "http://example.com", "", false),
                (
                    "http://example.net/adv",
                    "http://foo.example.com",
                    "",
                    false,
                ),
                (
                    "http://example.net/adv",
                    "http://www.foo.example.com",
                    "",
                    false,
                ),
            ],
        );

        check_option_rule(
            &["adv$domain=example.com|~foo.example.com,script"],
            &[
                (
                    "http://example.net/adv",
                    "http://example.com",
                    "script",
                    true,
                ),
                (
                    "http://example.net/adv",
                    "http://foo.example.com",
                    "script",
                    false,
                ),
                (
                    "http://example.net/adv",
                    "http://www.foo.example.com",
                    "script",
                    false,
                ),
                ("http://example.net/adv", "http://example.com", "", false),
                (
                    "http://example.net/adv",
                    "http://foo.example.com",
                    "",
                    false,
                ),
                (
                    "http://example.net/adv",
                    "http://www.foo.example.com",
                    "",
                    false,
                ),
            ],
        );
    }

    #[test]
    fn check_options_exception_notscript() {
        check_option_rule(
            &["adv", "@@advice.$~script"],
            &[
                ("http://example.com/advice.html", "", "other", false),
                ("http://example.com/advice.html", "", "script", true),
                ("http://example.com/advert.html", "", "other", true),
                ("http://example.com/advert.html", "", "script", true),
            ],
        );
    }

    #[test]
    fn check_options_third_party_flags() {
        // Single matching context domain to domain list
        check_option_rule(
            &["||mzstatic.com^$image,object-subrequest,domain=dailymotion.com"],
            &[(
                "http://www.dailymotion.com",
                "http://dailymotion.com",
                "",
                false,
            )],
        );

        // Third party flags work correctly
        check_option_rule(
            &["||s1.wp.com^$subdocument,third-party"],
            &[(
                "http://s1.wp.com/_static",
                "http://windsorstar.com",
                "",
                false,
            )],
        );

        // Third party flags work correctly
        check_option_rule(
            &["/scripts/ad."],
            &[(
                "http://a.fsdn.com/sd/js/scripts/ad.js?release_20160112",
                "http://slashdot.org",
                "script",
                true,
            )],
        );
    }
}

mod legacy_misc_tests {
    use adblock::filters::network::NetworkFilter;
    use adblock::request::Request;
    use adblock::Engine;

    #[test]
    fn demo_app() {
        // Demo app test
        let engine = Engine::from_rules(
            ["||googlesyndication.com/safeframe/$third-party"],
            Default::default(),
        );

        let request = Request::new(
            "http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html",
            "http://slashdot.org",
            "script",
        )
        .unwrap();
        assert!(engine.check_network_request(&request).matched)
    }

    #[test]
    fn host_anchored_filters_parse_correctly() {
        // Host anchor is calculated correctly
        let filter =
            NetworkFilter::parse("||test.com$third-party", false, Default::default()).unwrap();
        assert_eq!(filter.hostname, Some(String::from("test.com")));

        let filter =
            NetworkFilter::parse("||test.com/ok$third-party", false, Default::default()).unwrap();
        assert_eq!(filter.hostname, Some(String::from("test.com")));

        let filter = NetworkFilter::parse("||test.com/ok", false, Default::default()).unwrap();
        assert_eq!(filter.hostname, Some(String::from("test.com")));
    }

    #[test]
    fn serialization_tests() {
        let engine = Engine::from_rules_parametrised(
            [
                "||googlesyndication.com$third-party",
                "@@||googlesyndication.ca",
                "a$explicitcancel",
            ],
            Default::default(),
            true,
            false,
        ); // enable debugging and disable optimizations

        let serialized = engine.serialize().unwrap();
        let mut engine2 = Engine::default();
        engine2.deserialize(&serialized).unwrap();

        assert!(
            engine
                .check_network_request(
                    &Request::new(
                        "https://googlesyndication.com/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );
        assert!(
            engine2
                .check_network_request(
                    &Request::new(
                        "https://googlesyndication.com/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );
        assert!(
            !engine
                .check_network_request(
                    &Request::new(
                        "https://googleayndication.com/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );
        assert!(
            !engine2
                .check_network_request(
                    &Request::new(
                        "https://googleayndication.com/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );

        assert!(
            !engine
                .check_network_request(
                    &Request::new(
                        "https://googlesyndication.ca/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );
        assert!(
            !engine2
                .check_network_request(
                    &Request::new(
                        "https://googlesyndication.ca/script.js",
                        "https://example.com",
                        "script"
                    )
                    .unwrap()
                )
                .matched
        );
    }

    #[test]
    fn find_matching_filters() {
        let engine = Engine::from_rules_debug(
            [
                "||googlesyndication.com/safeframe/$third-party",
                "||brianbondy.com/ads",
            ],
            Default::default(),
        );

        let current_page_frame = "http://slashdot.org";
        let request_type = "script";

        // Test finds a match
        let request = Request::new(
            "http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html",
            current_page_frame,
            request_type,
        )
        .unwrap();
        let checked = engine.check_network_request(&request);
        assert!(checked.filter.is_some(), "Expected a fitler to match");
        assert!(
            checked.exception.is_none(),
            "Expected no exception to match"
        );
        let matched_filter = checked.filter.unwrap();
        assert_eq!(
            matched_filter,
            "||googlesyndication.com/safeframe/$third-party"
        );

        // Test when no filter is found, returns None
        let request = Request::new("http://ssafsdf.com", current_page_frame, request_type).unwrap();
        let checked = engine.check_network_request(&request);
        assert!(!checked.matched, "Expected url to pass");
        assert!(checked.filter.is_none(), "Expected no fitler to match");
        assert!(
            checked.exception.is_none(),
            "Expected no exception to match"
        );
        assert!(checked.redirect.is_none(), "Expected no redirect to match");
    }

    #[test]
    fn find_matching_filters_exceptions() {
        let engine = Engine::from_rules_debug(
            [
                "||googlesyndication.com/safeframe/$third-party",
                "||brianbondy.com/ads",
                "@@safeframe",
            ],
            Default::default(),
        );

        let current_page_frame = "http://slashdot.org";
        let request_type = "script";

        // Parse that it finds exception filters correctly
        let request = Request::new(
            "http://tpc.googlesyndication.com/safeframe/1-0-2/html/container.html",
            current_page_frame,
            request_type,
        )
        .unwrap();
        let checked = engine.check_network_request(&request);
        assert!(!checked.matched, "Expected url to pass");
        assert!(checked.filter.is_some(), "Expected a fitler to match");
        assert!(
            checked.exception.is_some(),
            "Expected no exception to match"
        );
        let matched_filter = checked.filter.unwrap();
        assert_eq!(
            matched_filter,
            "||googlesyndication.com/safeframe/$third-party"
        );
        let matched_exception = checked.exception.unwrap();
        assert_eq!(matched_exception, "@@safeframe");
    }

    #[test]
    fn matches_with_filter_info_preserves_important() {
        // exceptions have not effect if important filter matches
        let engine = Engine::from_rules_debug(
            ["||brianbondy.com^$important", "@@||brianbondy.com^"],
            Default::default(),
        );

        let request =
            Request::new("https://brianbondy.com/t", "https://test.com", "script").unwrap();
        let checked = engine.check_network_request(&request);

        assert!(checked.matched);
        assert!(checked.filter.is_some(), "Expected filter to match");
        let matched_filter = checked.filter.unwrap();
        assert_eq!(matched_filter, "||brianbondy.com^$important");
        assert!(
            checked.exception.is_none(),
            "Expected no exception to match"
        );
    }
}
