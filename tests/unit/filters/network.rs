
#[cfg(test)]
mod parse_tests {
    use super::super::*;

    #[derive(Debug, PartialEq)]
    struct NetworkFilterBreakdown {
        filter: Option<String>,
        hostname: Option<String>,
        opt_domains: Option<Vec<Hash>>,
        opt_not_domains: Option<Vec<Hash>>,
        modifier_option: Option<String>,

        // filter type
        is_exception: bool,
        is_hostname_anchor: bool,
        is_right_anchor: bool,
        is_left_anchor: bool,
        is_regex: bool,
        is_csp: bool,
        is_plain: bool,
        is_important: bool,

        // Options
        first_party: bool,
        from_network_types: bool,
        from_font: bool,
        from_image: bool,
        from_media: bool,
        from_object: bool,
        from_other: bool,
        from_ping: bool,
        from_script: bool,
        from_stylesheet: bool,
        from_subdocument: bool,
        from_websocket: bool,
        from_xml_http_request: bool,
        from_document: bool,
        match_case: bool,
        third_party: bool,
    }

    impl From<&NetworkFilter> for NetworkFilterBreakdown {
        fn from(filter: &NetworkFilter) -> NetworkFilterBreakdown {
            NetworkFilterBreakdown {
                filter: filter.filter.string_view(),
                hostname: filter.hostname.as_ref().cloned(),
                opt_domains: filter.opt_domains.as_ref().cloned(),
                opt_not_domains: filter.opt_not_domains.as_ref().cloned(),
                modifier_option: filter.modifier_option.as_ref().cloned(),

                // filter type
                is_exception: filter.is_exception(),
                is_hostname_anchor: filter.is_hostname_anchor(),
                is_right_anchor: filter.is_right_anchor(),
                is_left_anchor: filter.is_left_anchor(),
                is_regex: filter.is_regex(),
                is_csp: filter.is_csp(),
                is_plain: filter.is_plain(),
                is_important: filter.is_important(),

                // Options
                first_party: filter.first_party(),
                from_network_types: filter.mask.contains(NetworkFilterMask::FROM_NETWORK_TYPES),
                from_font: filter.mask.contains(NetworkFilterMask::FROM_FONT),
                from_image: filter.mask.contains(NetworkFilterMask::FROM_IMAGE),
                from_media: filter.mask.contains(NetworkFilterMask::FROM_MEDIA),
                from_object: filter.mask.contains(NetworkFilterMask::FROM_OBJECT),
                from_other: filter.mask.contains(NetworkFilterMask::FROM_OTHER),
                from_ping: filter.mask.contains(NetworkFilterMask::FROM_PING),
                from_script: filter.mask.contains(NetworkFilterMask::FROM_SCRIPT),
                from_stylesheet: filter.mask.contains(NetworkFilterMask::FROM_STYLESHEET),
                from_subdocument: filter.mask.contains(NetworkFilterMask::FROM_SUBDOCUMENT),
                from_websocket: filter.mask.contains(NetworkFilterMask::FROM_WEBSOCKET),
                from_xml_http_request: filter.mask.contains(NetworkFilterMask::FROM_XMLHTTPREQUEST),
                from_document: filter.mask.contains(NetworkFilterMask::FROM_DOCUMENT),
                match_case: filter.match_case(),
                third_party: filter.third_party(),
            }
        }
    }

    fn default_network_filter_breakdown() -> NetworkFilterBreakdown {
        NetworkFilterBreakdown {
            filter: None,
            hostname: None,
            opt_domains: None,
            opt_not_domains: None,
            modifier_option: None,

            // filter type
            is_exception: false,
            is_hostname_anchor: false,
            is_right_anchor: false,
            is_left_anchor: false,
            is_regex: false,
            is_csp: false,
            is_plain: false,
            is_important: false,

            // Options
            first_party: true,
            from_network_types: true,
            from_font: true,
            from_image: true,
            from_media: true,
            from_object: true,
            from_other: true,
            from_ping: true,
            from_script: true,
            from_stylesheet: true,
            from_subdocument: true,
            from_websocket: true,
            from_xml_http_request: true,
            from_document: false,
            match_case: false,
            third_party: true,
        }
    }

    #[test]
    // pattern
    fn parses_plain_pattern() {
        {
            let filter = NetworkFilter::parse("ads", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("ads"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("/ads/foo-", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("/ads/foo-"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("/ads/foo-$important", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("/ads/foo-"));
            defaults.is_plain = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("foo.com/ads$important", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_plain = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||pattern
    fn parses_hostname_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = None;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$important", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = None;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com/bar/baz$important", true, Default::default())
                    .unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||pattern|
    fn parses_hostname_right_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = None;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com|$important", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = None;
            defaults.is_plain = true;
            defaults.is_important = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com/bar/baz|$important", true, Default::default())
                    .unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^bar/*baz|$important", true, Default::default())
                    .unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar/*baz"));
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |pattern
    fn parses_left_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("|foo.com/bar/baz", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/bar/baz"));
            defaults.is_plain = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("|foo.com^bar/*baz", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com^bar/*baz"));
            defaults.is_regex = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |pattern|
    fn parses_left_right_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com|", true, Default::default()).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com/bar|", true, Default::default()).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/bar"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com*bar^|", true, Default::default()).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_regex = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||regexp
    fn parses_hostname_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com*bar^", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^bar*/baz^", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar*/baz^"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||regexp|
    fn parses_hostname_right_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com*bar^|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^bar*/baz^|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar*/baz^"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |regexp
    fn parses_hostname_left_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com*bar^", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("|foo.com^bar*/baz^", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com^bar*/baz^"));
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |regexp|
    fn parses_hostname_left_right_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com*bar^|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter =
                NetworkFilter::parse("|foo.com^bar*/baz^|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com^bar*/baz^"));
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // @@pattern
    fn parses_exception_pattern() {
        {
            let filter = NetworkFilter::parse("@@ads", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("ads"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@||foo.com/ads", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("/ads"));
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@|foo.com/ads", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@|foo.com/ads|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@foo.com/ads|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("@@||foo.com/ads|", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("/ads"));
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
    }

    // Options

    #[test]
    fn accepts_any_content_type() {
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_network_types = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$first-party", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_network_types = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.first_party = true;
            defaults.third_party = false;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$third-party", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_network_types = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.first_party = false;
            defaults.third_party = true;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=test.com", true, Default::default())
                    .unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_network_types = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.opt_domains = Some(vec![utils::fast_hash("test.com")]);

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=test.com", true, Default::default())
                    .unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_network_types = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.opt_domains = Some(vec![utils::fast_hash("test.com")]);

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
    }

    #[test]
    fn parses_important() {
        {
            let filter =
                NetworkFilter::parse("||foo.com$important", true, Default::default()).unwrap();
            assert_eq!(filter.is_important(), true);
        }
        {
            // parses ~important
            let filter = NetworkFilter::parse("||foo.com$~important", true, Default::default());
            assert_eq!(filter.err(), Some(NetworkFilterError::NegatedImportant));
        }
        {
            // defaults to false
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.is_important(), false);
        }
    }

    #[test]
    fn parses_csp() {
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.modifier_option, None);
        }
        {
            // parses simple CSP
            let filter =
                NetworkFilter::parse(r#"||foo.com$csp=self bar """#, true, Default::default())
                    .unwrap();
            assert_eq!(filter.is_csp(), true);
            assert_eq!(filter.modifier_option, Some(String::from(r#"self bar """#)));
        }
        {
            // parses empty CSP
            let filter = NetworkFilter::parse("||foo.com$csp", true, Default::default()).unwrap();
            assert_eq!(filter.is_csp(), true);
            assert_eq!(filter.modifier_option, None);
        }
        {
            // CSP mixed with content type is an error
            let filter = NetworkFilter::parse(
                r#"||foo.com$domain=foo|bar,csp=self bar "",image"#,
                true,
                Default::default(),
            );
            assert_eq!(filter.err(), Some(NetworkFilterError::CspWithContentType));
        }
    }

    #[test]
    fn parses_domain() {
        // parses domain
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=bar.com", true, Default::default()).unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("bar.com")]));
            assert_eq!(filter.opt_not_domains, None);
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=bar.com|baz.com", true, Default::default())
                    .unwrap();
            let mut domains = vec![utils::fast_hash("bar.com"), utils::fast_hash("baz.com")];
            domains.sort_unstable();
            assert_eq!(filter.opt_domains, Some(domains));
            assert_eq!(filter.opt_not_domains, None);
        }

        // parses ~domain
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=~bar.com", true, Default::default())
                    .unwrap();
            assert_eq!(filter.opt_domains, None);
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("bar.com")])
            );
        }
        {
            let filter = NetworkFilter::parse(
                "||foo.com$domain=~bar.com|~baz.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert_eq!(filter.opt_domains, None);
            let mut domains = vec![utils::fast_hash("bar.com"), utils::fast_hash("baz.com")];
            domains.sort_unstable();
            assert_eq!(filter.opt_not_domains, Some(domains));
        }
        // parses domain and ~domain
        {
            let filter = NetworkFilter::parse(
                "||foo.com$domain=~bar.com|baz.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("baz.com")]));
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("bar.com")])
            );
        }
        {
            let filter = NetworkFilter::parse(
                "||foo.com$domain=bar.com|~baz.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("bar.com")]));
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("baz.com")])
            );
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=foo|~bar|baz", true, Default::default())
                    .unwrap();
            let mut domains = vec![utils::fast_hash("foo"), utils::fast_hash("baz")];
            domains.sort();
            assert_eq!(filter.opt_domains, Some(domains));
            assert_eq!(filter.opt_not_domains, Some(vec![utils::fast_hash("bar")]));
        }
        // defaults to no constraint
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.opt_domains, None);
            assert_eq!(filter.opt_not_domains, None);
        }
        // `from` is an alias for `domain`
        {
            let filter =
                NetworkFilter::parse("||foo.com$from=bar.com", true, Default::default()).unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("bar.com")]));
            assert_eq!(filter.opt_not_domains, None);
        }
        {
            let filter = NetworkFilter::parse(
                r"||video.twimg.com/ext_tw_video/*/*.m3u8$domain=/^i[a-z]*\.strmrdr[a-z]+\..*/",
                true,
                Default::default(),
            );
            assert_eq!(filter.err(), Some(NetworkFilterError::NoSupportedDomains));
        }
    }

    #[test]
    fn parses_redirects() {
        // parses redirect
        {
            let filter =
                NetworkFilter::parse("||foo.com$redirect=bar.js", true, Default::default())
                    .unwrap();
            assert_eq!(filter.modifier_option, Some(String::from("bar.js")));
        }
        {
            let filter =
                NetworkFilter::parse("$redirect=bar.js", true, Default::default()).unwrap();
            assert_eq!(filter.modifier_option, Some(String::from("bar.js")));
        }
        // parses ~redirect
        {
            // ~redirect is not a valid option
            let filter = NetworkFilter::parse("||foo.com$~redirect", true, Default::default());
            assert_eq!(filter.err(), Some(NetworkFilterError::NegatedRedirection));
        }
        // parses redirect without a value
        {
            // Not valid
            let filter = NetworkFilter::parse("||foo.com$redirect", true, Default::default());
            assert_eq!(filter.err(), Some(NetworkFilterError::EmptyRedirection));
        }
        {
            let filter = NetworkFilter::parse("||foo.com$redirect=", true, Default::default());
            assert_eq!(filter.err(), Some(NetworkFilterError::EmptyRedirection))
        }
        // defaults to false
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.modifier_option, None);
        }
    }

    #[test]
    fn parses_removeparam() {
        {
            let filter = NetworkFilter::parse("||foo.com^$removeparam", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter = NetworkFilter::parse("$~removeparam=test", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter =
                NetworkFilter::parse("@@||foo.com^$removeparam=test", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter = NetworkFilter::parse("||foo.com^$removeparam=", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter = NetworkFilter::parse(
                "||foo.com^$removeparam=test,redirect=test",
                true,
                Default::default(),
            );
            assert!(filter.is_err());
        }
        {
            let filter = NetworkFilter::parse(
                "||foo.com^$removeparam=test,removeparam=test2",
                true,
                Default::default(),
            );
            assert!(filter.is_err());
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^$removeparam=𝐔𝐍𝐈𝐂𝐎𝐃𝐄🧋", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^$removeparam=/abc.*/", true, Default::default());
            assert_eq!(filter, Err(NetworkFilterError::RemoveparamRegexUnsupported));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com^$removeparam=test", true, Default::default())
                    .unwrap();
            assert!(filter.is_removeparam());
            assert_eq!(filter.modifier_option, Some("test".into()));
        }
    }

    #[test]
    fn parses_match_case() {
        // match-case on non-regex rules is invalid
        {
            assert!(
                NetworkFilter::parse("||foo.com$match-case", true, Default::default()).is_err()
            );
        }
        {
            assert!(
                NetworkFilter::parse("||foo.com$image,match-case", true, Default::default())
                    .is_err()
            );
        }
        {
            assert!(NetworkFilter::parse(
                "||foo.com$media,match-case,image",
                true,
                Default::default()
            )
            .is_err());
        }
        // match-case on regex rules is ok
        {
            let filter = NetworkFilter::parse(
                r#"/foo[0-9]*\.com/$media,match-case,image"#,
                true,
                Default::default(),
            )
            .unwrap();
            assert_eq!(filter.match_case(), true);
        }
        {
            let filter = NetworkFilter::parse(r#"/^https?:\/\/[a-z]{8,15}\.top\/[-a-z]{4,}\.css\?aHR0c[\/0-9a-zA-Z]{33,}=?=?\$/$css,3p,match-case"#, true, Default::default()).unwrap();
            assert_eq!(filter.match_case(), true);
        }

        // parses ~match-case
        {
            // ~match-case is not supported
            let filter = NetworkFilter::parse("||foo.com$~match-case", true, Default::default());
            assert_eq!(
                filter.err(),
                Some(NetworkFilterError::NegatedOptionMatchCase)
            );
        }

        // defaults to false
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.match_case(), false)
        }
    }

    #[test]
    fn parses_first_party() {
        // parses first-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$first-party", true, Default::default())
                .unwrap()
                .first_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com$first-party", true, Default::default())
                .unwrap()
                .first_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com|$first-party", true, Default::default())
                .unwrap()
                .first_party(),
            true
        );
        // parses ~first-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$~first-party", true, Default::default())
                .unwrap()
                .first_party(),
            false
        );
        assert_eq!(
            NetworkFilter::parse(
                "||foo.com$first-party,~first-party",
                true,
                Default::default()
            )
            .unwrap()
            .first_party(),
            false
        );
        // defaults to true
        assert_eq!(
            NetworkFilter::parse("||foo.com", true, Default::default())
                .unwrap()
                .first_party(),
            true
        );
    }

    #[test]
    fn parses_third_party() {
        // parses third-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$third-party", true, Default::default())
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com$third-party", true, Default::default())
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com|$third-party", true, Default::default())
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("||foo.com$~first-party", true, Default::default())
                .unwrap()
                .third_party(),
            true
        );
        // parses ~third-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$~third-party", true, Default::default())
                .unwrap()
                .third_party(),
            false
        );
        assert_eq!(
            NetworkFilter::parse(
                "||foo.com$first-party,~third-party",
                true,
                Default::default()
            )
            .unwrap()
            .third_party(),
            false
        );
        // defaults to true
        assert_eq!(
            NetworkFilter::parse("||foo.com", true, Default::default())
                .unwrap()
                .third_party(),
            true
        );
    }

    #[test]
    fn parses_generic_hide() {
        {
            let filter = NetworkFilter::parse("||foo.com$generichide", true, Default::default());
            assert!(filter.is_err());
        }
        {
            let filter =
                NetworkFilter::parse("@@||foo.com$generichide", true, Default::default()).unwrap();
            assert_eq!(filter.is_exception(), true);
            assert_eq!(filter.is_generic_hide(), true);
        }
        {
            let filter =
                NetworkFilter::parse("@@||foo.com|$generichide", true, Default::default()).unwrap();
            assert_eq!(filter.is_exception(), true);
            assert_eq!(filter.is_generic_hide(), true);
        }
        {
            let filter = NetworkFilter::parse(
                "@@$generichide,domain=example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert_eq!(filter.is_generic_hide(), true);
            let breakdown = NetworkFilterBreakdown::from(&filter);
            assert_eq!(
                breakdown.opt_domains,
                Some(vec![utils::fast_hash("example.com")])
            );
        }
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert_eq!(filter.is_generic_hide(), false);
        }
    }

    #[test]
    fn parses_hosts_style() {
        {
            let filter = NetworkFilter::parse_hosts_style("example.com", true).unwrap();
            assert!(filter.raw_line.is_some());
            assert_eq!(*filter.raw_line.clone().unwrap(), "||example.com^");
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some("example.com".to_string());
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_right_anchor = true;
            defaults.from_document = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse_hosts_style("www.example.com", true).unwrap();
            assert!(filter.raw_line.is_some());
            assert_eq!(*filter.raw_line.clone().unwrap(), "||example.com^");
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some("example.com".to_string());
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_right_anchor = true;
            defaults.from_document = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse_hosts_style("malware.example.com", true).unwrap();
            assert!(filter.raw_line.is_some());
            assert_eq!(*filter.raw_line.clone().unwrap(), "||malware.example.com^");
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some("malware.example.com".to_string());
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_right_anchor = true;
            defaults.from_document = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    fn handles_unsupported_options() {
        let options = vec!["genericblock", "inline-script", "popunder", "popup", "woot"];

        for option in options {
            let filter =
                NetworkFilter::parse(&format!("||foo.com${}", option), true, Default::default());
            assert!(filter.err().is_some());
        }
    }

    #[test]
    fn handles_content_type_options() {
        let options = vec![
            "font",
            "image",
            "media",
            "object",
            "object-subrequest",
            "other",
            "ping",
            "script",
            "stylesheet",
            "subdocument",
            "websocket",
            "xmlhttprequest",
            "xhr",
        ];

        fn set_all_options(breakdown: &mut NetworkFilterBreakdown, value: bool) {
            breakdown.from_font = value;
            breakdown.from_image = value;
            breakdown.from_media = value;
            breakdown.from_object = value;
            breakdown.from_other = value;
            breakdown.from_ping = value;
            breakdown.from_script = value;
            breakdown.from_stylesheet = value;
            breakdown.from_subdocument = value;
            breakdown.from_websocket = value;
            breakdown.from_xml_http_request = value;
        }

        fn set_option(option: &str, breakdown: &mut NetworkFilterBreakdown, value: bool) {
            match option {
                "font" => breakdown.from_font = value,
                "image" => breakdown.from_image = value,
                "media" => breakdown.from_media = value,
                "object" => breakdown.from_object = value,
                "object-subrequest" => breakdown.from_object = value,
                "other" => breakdown.from_other = value,
                "ping" => breakdown.from_ping = value,
                "script" => breakdown.from_script = value,
                "stylesheet" => breakdown.from_stylesheet = value,
                "subdocument" => breakdown.from_subdocument = value,
                "websocket" => breakdown.from_websocket = value,
                "xmlhttprequest" => breakdown.from_xml_http_request = value,
                "xhr" => breakdown.from_xml_http_request = value,
                _ => unreachable!(),
            }
        }

        for option in options {
            // positive
            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com${}", option),
                    true,
                    Default::default(),
                )
                .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com$object,{}", option),
                    true,
                    Default::default(),
                )
                .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                set_option("object", &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com$domain=bar.com,{}", option),
                    true,
                    Default::default(),
                )
                .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                defaults.opt_domains = Some(vec![utils::fast_hash("bar.com")]);
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            // negative
            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com$~{}", option),
                    true,
                    Default::default(),
                )
                .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, true);
                set_option(&option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com${},~{}", option, option),
                    true,
                    Default::default(),
                )
                .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, true);
                set_option(&option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }
            // default - positive
            {
                let filter =
                    NetworkFilter::parse(&format!("||foo.com"), true, Default::default()).unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = true;
                set_all_options(&mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }
        }
    }

    #[test]
    fn binary_serialization_works() {
        use rmp_serde::{Deserializer, Serializer};
        {
            let filter =
                NetworkFilter::parse("||foo.com/bar/baz$important", true, Default::default())
                    .unwrap();

            let mut encoded = Vec::new();
            filter
                .serialize(&mut Serializer::new(&mut encoded))
                .unwrap();
            let mut de = Deserializer::new(&encoded[..]);
            let decoded: NetworkFilter = Deserialize::deserialize(&mut de).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&decoded))
        }
        {
            let filter = NetworkFilter::parse("||foo.com*bar^", true, Default::default()).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;

            let mut encoded = Vec::new();
            filter
                .serialize(&mut Serializer::new(&mut encoded))
                .unwrap();
            let mut de = Deserializer::new(&encoded[..]);
            let decoded: NetworkFilter = Deserialize::deserialize(&mut de).unwrap();

            assert_eq!(defaults, NetworkFilterBreakdown::from(&decoded));
            assert_eq!(RegexManager::default().matches(&decoded, "bar/"), true);
        }
    }

    #[test]
    fn parse_empty_host_anchor_exception() {
        let filter_parsed =
            NetworkFilter::parse("@@||$domain=auth.wi-fi.ru", true, Default::default());
        assert!(filter_parsed.is_ok());

        let filter = filter_parsed.unwrap();

        let mut defaults = default_network_filter_breakdown();

        defaults.hostname = Some(String::from(""));
        defaults.is_hostname_anchor = true;
        defaults.is_exception = true;
        defaults.is_plain = true;
        defaults.from_network_types = true;
        defaults.opt_domains = Some(vec![utils::fast_hash("auth.wi-fi.ru")]);
        assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
    }
}

#[cfg(test)]
mod match_tests {
    use super::super::*;

    #[test]
    fn is_anchored_by_hostname_works() {
        // matches empty hostname
        assert_eq!(is_anchored_by_hostname("", "foo.com", false), true);

        // does not match when filter hostname is longer than hostname
        assert_eq!(
            is_anchored_by_hostname("bar.foo.com", "foo.com", false),
            false
        );
        assert_eq!(is_anchored_by_hostname("b", "", false), false);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.co", false), false);

        // does not match if there is not match
        assert_eq!(is_anchored_by_hostname("bar", "foo.com", false), false);

        // ## prefix match
        // matches exact match
        assert_eq!(is_anchored_by_hostname("", "", false), true);
        assert_eq!(is_anchored_by_hostname("f", "f", false), true);
        assert_eq!(is_anchored_by_hostname("foo", "foo", false), true);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com", ".com", false), true);
        assert_eq!(is_anchored_by_hostname("com.", "com.", false), true);

        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname("foo.", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".foo", ".foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".foo.", ".foo.com", false), true);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname("foo.com.", "foo.com.", false), true);
        assert_eq!(
            is_anchored_by_hostname(".foo.com.", ".foo.com.", false),
            true
        );
        assert_eq!(is_anchored_by_hostname(".foo.com", ".foo.com", false), true);

        assert_eq!(
            is_anchored_by_hostname("foo.bar", "foo.bar.com", false),
            true
        );
        assert_eq!(
            is_anchored_by_hostname("foo.bar.", "foo.bar.com", false),
            true
        );

        // does not match partial prefix
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foobar.com", false), false);
        assert_eq!(is_anchored_by_hostname("fo", "foo.com", false), false);
        assert_eq!(is_anchored_by_hostname(".foo", "foobar.com", false), false);

        // Multiple labels
        assert_eq!(
            is_anchored_by_hostname("foo.bar", "foo.barbaz.com", false),
            false
        );
        assert_eq!(
            is_anchored_by_hostname(".foo.bar", ".foo.barbaz.com", false),
            false
        );

        // ## suffix match
        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com.", "foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname("com.", "foo.com.", false), true);

        // Multiple labels
        assert_eq!(
            is_anchored_by_hostname("foo.com.", ".foo.com.", false),
            true
        );
        assert_eq!(is_anchored_by_hostname("foo.com", ".foo.com", false), true);

        // does not match partial
        // Single label
        assert_eq!(is_anchored_by_hostname("om", "foo.com", false), false);
        assert_eq!(is_anchored_by_hostname("com", "foocom", false), false);

        // Multiple labels
        assert_eq!(
            is_anchored_by_hostname("foo.bar.com", "baz.bar.com", false),
            false
        );
        assert_eq!(
            is_anchored_by_hostname("fo.bar.com", "foo.bar.com", false),
            false
        );
        assert_eq!(
            is_anchored_by_hostname(".fo.bar.com", "foo.bar.com", false),
            false
        );
        assert_eq!(
            is_anchored_by_hostname("bar.com", "foobar.com", false),
            false
        );
        assert_eq!(
            is_anchored_by_hostname(".bar.com", "foobar.com", false),
            false
        );

        // ## infix match
        // matches partial
        assert_eq!(is_anchored_by_hostname("bar", "foo.bar.com", false), true);
        assert_eq!(is_anchored_by_hostname("bar.", "foo.bar.com", false), true);
        assert_eq!(is_anchored_by_hostname(".bar.", "foo.bar.com", false), true);
    }

    fn filter_match_url(filter: &str, url: &str, matching: bool) {
        let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
        let request = request::Request::new(url, "https://example.com", "other").unwrap();

        assert!(
            network_filter.matches_test(&request) == matching,
            "Expected match={} for {} {:?} on {}",
            matching,
            filter,
            network_filter,
            url
        );
    }

    fn hosts_filter_match_url(filter: &str, url: &str, matching: bool) {
        let network_filter = NetworkFilter::parse_hosts_style(filter, true).unwrap();
        let request = request::Request::new(url, "https://example.com", "other").unwrap();

        assert!(
            network_filter.matches_test(&request) == matching,
            "Expected match={} for {} {:?} on {}",
            matching,
            filter,
            network_filter,
            url
        );
    }

    #[test]
    // pattern
    fn check_pattern_plain_filter_filter_works() {
        filter_match_url("foo", "https://bar.com/foo", true);
        filter_match_url("foo", "https://bar.com/baz/foo", true);
        filter_match_url("foo", "https://bar.com/q=foo/baz", true);
        filter_match_url("foo", "https://foo.com", true);
        filter_match_url("-foo-", "https://bar.com/baz/42-foo-q", true);
        filter_match_url("&fo.o=+_-", "https://bar.com?baz=42&fo.o=+_-", true);
        filter_match_url("foo/bar/baz", "https://bar.com/foo/bar/baz", true);
        filter_match_url("com/bar/baz", "https://bar.com/bar/baz", true);
        filter_match_url("https://bar.com/bar/baz", "https://bar.com/bar/baz", true);
    }

    #[test]
    // ||pattern
    fn check_pattern_hostname_anchor_filter_works() {
        filter_match_url("||foo.com", "https://foo.com/bar", true);
        filter_match_url("||foo.com/bar", "https://foo.com/bar", true);
        filter_match_url("||foo", "https://foo.com/bar", true);
        filter_match_url("||foo", "https://baz.foo.com/bar", true);
        filter_match_url("||foo", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz.", "https://foo.baz.com/bar", true);

        filter_match_url("||foo.baz.com^", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz^", "https://foo.baz.com/bar", false);

        filter_match_url("||foo", "https://baz.com", false);
        filter_match_url("||foo", "https://foo-bar.baz.com/bar", false);
        filter_match_url("||foo.com", "https://foo.de", false);
        filter_match_url("||foo.com", "https://bar.foo.de", false);
        filter_match_url("||s.foo.com", "https://substring.s.foo.com", true);
        filter_match_url("||s.foo.com", "https://substrings.foo.com", false);
    }

    #[test]
    fn check_hosts_style_works() {
        hosts_filter_match_url("foo.com", "https://foo.com/bar", true);
        hosts_filter_match_url("foo.foo.com", "https://foo.com/bar", false);
        hosts_filter_match_url("www.foo.com", "https://foo.com/bar", true);
        hosts_filter_match_url("com.foo", "https://foo.baz.com/bar", false);
        hosts_filter_match_url("foo.baz", "https://foo.baz.com/bar", false);

        hosts_filter_match_url("foo.baz.com", "https://foo.baz.com/bar", true);
        hosts_filter_match_url("foo.baz", "https://foo.baz.com/bar", false);

        hosts_filter_match_url("foo.com", "https://baz.com", false);
        hosts_filter_match_url("bar.baz.com", "https://foo-bar.baz.com/bar", false);
        hosts_filter_match_url("foo.com", "https://foo.de", false);
        hosts_filter_match_url("foo.com", "https://bar.foo.de", false);
    }

    #[test]
    // ||pattern|
    fn check_pattern_hostname_right_anchor_filter_works() {
        filter_match_url("||foo.com|", "https://foo.com", true);
        filter_match_url("||foo.com/bar|", "https://foo.com/bar", true);

        filter_match_url("||foo.com/bar|", "https://foo.com/bar/baz", false);
        filter_match_url("||foo.com/bar|", "https://foo.com/", false);
        filter_match_url("||bar.com/bar|", "https://foo.com/", false);
    }

    #[test]
    // pattern|
    fn check_pattern_right_anchor_filter_works() {
        filter_match_url("foo.com", "https://foo.com", true);
        filter_match_url("foo|", "https://bar.com/foo", true);
        filter_match_url("foo|", "https://bar.com/foo/", false);
        filter_match_url("foo|", "https://bar.com/foo/baz", false);
    }

    #[test]
    // |pattern
    fn check_pattern_left_anchor_filter_works() {
        filter_match_url("|http", "http://foo.com", true);
        filter_match_url("|http", "https://foo.com", true);
        filter_match_url("|https://", "https://foo.com", true);

        filter_match_url("https", "http://foo.com", false);
    }

    #[test]
    // |pattern|
    fn check_pattern_left_right_anchor_filter_works() {
        filter_match_url("|https://foo.com|", "https://foo.com", true);
    }

    #[test]
    // ||pattern + left-anchor
    fn check_pattern_hostname_left_anchor_filter_works() {
        filter_match_url("||foo.com^test", "https://foo.com/test", true);
        filter_match_url("||foo.com/test", "https://foo.com/test", true);
        filter_match_url("||foo.com^test", "https://foo.com/tes", false);
        filter_match_url("||foo.com/test", "https://foo.com/tes", false);

        filter_match_url("||foo.com^", "https://foo.com/test", true);

        filter_match_url("||foo.com/test*bar", "https://foo.com/testbar", true);
        filter_match_url("||foo.com^test*bar", "https://foo.com/testbar", true);
    }

    #[test]
    // ||hostname^*/pattern
    fn check_pattern_hostname_anchor_regex_filter_works() {
        filter_match_url("||foo.com^*/bar", "https://foo.com/bar", false);
        filter_match_url("||com^*/bar", "https://foo.com/bar", false);
        filter_match_url("||foo^*/bar", "https://foo.com/bar", false);

        // @see https://github.com/cliqz-oss/adblocker/issues/29
        filter_match_url("||foo.co^aaa/", "https://bar.foo.com/bbb/aaa/", false);
        filter_match_url("||foo.com^aaa/", "https://bar.foo.com/bbb/aaa/", false);

        filter_match_url("||com*^bar", "https://foo.com/bar", true);
        filter_match_url("||foo.com^bar", "https://foo.com/bar", true);
        filter_match_url("||com^bar", "https://foo.com/bar", true);
        filter_match_url("||foo*^bar", "https://foo.com/bar", true);
        filter_match_url("||foo*/bar", "https://foo.com/bar", true);
        filter_match_url("||foo*com/bar", "https://foo.com/bar", true);
        filter_match_url("||foo2*com/bar", "https://foo2.com/bar", true);
        filter_match_url("||foo*com*/bar", "https://foo.com/bar", true);
        filter_match_url("||foo*com*^bar", "https://foo.com/bar", true);
        filter_match_url("||*foo*com*^bar", "https://foo.com/bar", true);
        filter_match_url("||*/bar", "https://foo.com/bar", true);
        filter_match_url("||*^bar", "https://foo.com/bar", true);
        filter_match_url("||*com/bar", "https://foo.com/bar", true);
        filter_match_url("||*.com/bar", "https://foo.com/bar", true);
        filter_match_url("||*foo.com/bar", "https://foo.com/bar", true);
        filter_match_url("||*com/bar", "https://foo.com/bar", true);
        filter_match_url("||*com*/bar", "https://foo.com/bar", true);
        filter_match_url("||*com*^bar", "https://foo.com/bar", true);
    }

    #[test]
    fn check_pattern_hostname_anchor_regex_filter_works_realisitic() {
        filter_match_url(
            "||vimeo.com^*?type=",
            "https://vimeo.com/ablincoln/fatal_attraction?type=pageview&target=%2F193641463",
            true,
        );
    }

    #[test]
    fn check_pattern_hostname_left_right_anchor_regex_filter_works() {
        filter_match_url("||geo*.hltv.org^", "https://geo2.hltv.org/rekl13.php", true);
        filter_match_url(
            "||www*.swatchseries.to^",
            "https://www1.swatchseries.to/sw.js",
            true,
        );
        filter_match_url("||imp*.tradedoubler.com^", "https://impde.tradedoubler.com/imp?type(js)g(22608602)a(1725113)epi(30148500144427100033372010772028)preurl(https://pixel.mathtag.com/event/js?mt_id=1160537&mt_adid=166882&mt_exem=&mt_excl=&v1=&v2=&v3=&s1=&s2=&s3=&mt_nsync=1&redirect=https%3A%2F%2Fad28.ad-srv.net%2Fc%2Fczqwm6dm6kagr2j%3Ftprde%3D)768489806", true);
    }

    #[test]
    fn check_pattern_exception_works() {
        {
            let filter = "@@||fastly.net/ad2/$image,script,xmlhttprequest";
            let url = "https://0914.global.ssl.fastly.net/ad2/script/x.js?cb=1549980040838";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let request =
                request::Request::new(url, "https://www.gamespot.com/metro-exodus/", "script")
                    .unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            let filter = "@@||swatchseries.to/public/js/edit-show.js$script,domain=swatchseries.to";
            let url = "https://www1.swatchseries.to/public/js/edit-show.js";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let request = request::Request::new(
                url,
                "https://www1.swatchseries.to/serie/roswell_new_mexico",
                "script",
            )
            .unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    fn check_pattern_match_case() {
        filter_match_url(
            r#"/BannerAd[0-9]/$match-case"#,
            "https://example.com/BannerAd0.gif",
            true,
        );
        filter_match_url(
            r#"/BannerAd[0-9]/$match-case"#,
            "https://example.com/bannerad0.gif",
            false,
        );
    }

    #[test]
    fn check_ws_vs_http_matching() {
        let network_filter =
            NetworkFilter::parse("|ws://$domain=4shared.com", true, Default::default()).unwrap();

        assert!(network_filter.matches_test(
            &request::Request::new("ws://example.com", "https://4shared.com", "websocket").unwrap()
        ));
        assert!(network_filter.matches_test(
            &request::Request::new("wss://example.com", "https://4shared.com", "websocket")
                .unwrap()
        ));
        assert!(!network_filter.matches_test(
            &request::Request::new("http://example.com", "https://4shared.com", "script").unwrap()
        ));
        assert!(!network_filter.matches_test(
            &request::Request::new("https://example.com", "https://4shared.com", "script").unwrap()
        ));

        // The `ws://` and `wss://` protocols should be used, rather than the resource type.
        assert!(network_filter.matches_test(
            &request::Request::new("ws://example.com", "https://4shared.com", "script").unwrap()
        ));
        assert!(network_filter.matches_test(
            &request::Request::new("wss://example.com", "https://4shared.com", "script").unwrap()
        ));
        assert!(!network_filter.matches_test(
            &request::Request::new("http://example.com", "https://4shared.com", "websocket")
                .unwrap()
        ));
        assert!(!network_filter.matches_test(
            &request::Request::new("https://example.com", "https://4shared.com", "websocket")
                .unwrap()
        ));
    }

    #[test]
    // options
    fn check_options_works() {
        // cpt test
        {
            let network_filter =
                NetworkFilter::parse("||foo$image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "image").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "script").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "script").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }

        // ~third-party
        {
            let network_filter =
                NetworkFilter::parse("||foo$~third-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~third-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // ~first-party
        {
            let network_filter =
                NetworkFilter::parse("||foo$~first-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~first-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // opt-domain
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=foo.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=foo.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // opt-not-domain
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=~bar.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=~bar.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }
    }

    #[test]
    fn check_domain_option_subsetting_works() {
        {
            let network_filter = NetworkFilter::parse(
                "adv$domain=example.com|~foo.example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
        }
        {
            let network_filter = NetworkFilter::parse(
                "adv$domain=~example.com|~foo.example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
        }
        {
            let network_filter = NetworkFilter::parse(
                "adv$domain=example.com|foo.example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
        }
        {
            let network_filter = NetworkFilter::parse(
                "adv$domain=~example.com|foo.example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                ) == false
            );
        }
        {
            let network_filter =
                NetworkFilter::parse("adv$domain=com|~foo.com", true, Default::default()).unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://com", "").unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.com", "").unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://subfoo.foo.com", "")
                        .unwrap()
                ) == false
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.com", "").unwrap()
                ) == true
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://co.uk", "").unwrap()
                ) == false
            );
        }
    }

    #[test]
    fn check_unicode_handled() {
        filter_match_url(
            "||firstrowsports.li/frame/",
            "https://firstrowsports.li/frame/bar",
            true,
        );
        filter_match_url(
            "||fırstrowsports.eu/pu/",
            "https://fırstrowsports.eu/pu/foo",
            true,
        );
        filter_match_url(
            "||fırstrowsports.eu/pu/",
            "https://xn--frstrowsports-39b.eu/pu/foo",
            true,
        );

        filter_match_url("||atđhe.net/pu/", "https://atđhe.net/pu/foo", true);
        filter_match_url("||atđhe.net/pu/", "https://xn--athe-1ua.net/pu/foo", true);

        filter_match_url("foo", "https://example.com/Ѥ/foo", true);
        filter_match_url("Ѥ", "https://example.com/Ѥ/foo", true);
    }

    #[test]
    fn check_regex_escaping_handled() {
        // A few rules that are not correctly escaped for rust Regex
        {
            // regex escaping "\/" unrecognised
            let filter =
                r#"/^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/$domain=123movies.com|1337x.to"#;
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://bit.ly/bar/";
            let source = "http://123movies.com";
            let request = request::Request::new(url, source, "").unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            // regex escaping "\:" unrecognised
            let filter = r#"/\:\/\/data.*\.com\/[a-zA-Z0-9]{30,}/$third-party,xmlhttprequest"#;
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer";
            let source = "http://123movies.com";
            let request = request::Request::new(url, source, "xmlhttprequest").unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        //
        {
            let filter = r#"/\.(accountant|bid|click|club|com|cricket|date|download|faith|link|loan|lol|men|online|party|racing|review|science|site|space|stream|top|trade|webcam|website|win|xyz|com)\/(([0-9]{2,9})(\.|\/)(css|\?)?)$/$script,stylesheet,third-party,xmlhttprequest"#;
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://hello.club/123.css";
            let source = "http://123movies.com";
            let request = request::Request::new(url, source, "stylesheet").unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    #[ignore] // Not going to handle lookaround regexes
    #[cfg(feature = "regex-debug-info")]
    fn check_lookaround_regex_handled() {
        {
            let filter = r#"/^https?:\/\/([0-9a-z\-]+\.)?(9anime|animeland|animenova|animeplus|animetoon|animewow|gamestorrent|goodanime|gogoanime|igg-games|kimcartoon|memecenter|readcomiconline|toonget|toonova|watchcartoononline)\.[a-z]{2,4}\/(?!([Ee]xternal|[Ii]mages|[Ss]cripts|[Uu]ploads|ac|ajax|assets|combined|content|cov|cover|(img\/bg)|(img\/icon)|inc|jwplayer|player|playlist-cat-rss|static|thumbs|wp-content|wp-includes)\/)(.*)/$image,other,script,~third-party,xmlhttprequest,domain=~animeland.hu"#;
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer";
            let source = "http://123movies.com";
            let request = request::Request::new(url, source, "script").unwrap();
            let mut regex_manager = RegexManager::default();
            assert!(regex_manager.get_compiled_regex_count() == 0);
            assert!(
                network_filter.matches(&request, &mut regex_manager) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
            assert!(regex_manager.get_compiled_regex_count() == 1);
        }
    }

    #[test]
    fn check_empty_host_anchor_matches() {
        {
            let filter = "||$domain=auth.wi-fi.ru";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://example.com/ad.js";
            let source = "http://auth.wi-fi.ru";
            let request = request::Request::new(url, source, "script").unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            let filter = "@@||$domain=auth.wi-fi.ru";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://example.com/ad.js";
            let source = "http://auth.wi-fi.ru";
            let request = request::Request::new(url, source, "script").unwrap();
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    fn check_url_path_regex_matches() {
        {
            let filter = "@@||www.google.com/aclk?*&adurl=$document,~third-party";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=";
            let source = "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=";
            let request = request::Request::new(url, source, "document").unwrap();
            assert!(!request.is_third_party);
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            let filter = "@@||www.google.*/aclk?$first-party";
            let network_filter = NetworkFilter::parse(filter, true, Default::default()).unwrap();
            let url = "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=";
            let source = "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=";
            let request = request::Request::new(url, source, "main_frame").unwrap();
            assert!(!request.is_third_party);
            assert!(
                network_filter.matches_test(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    fn check_get_url_after_hostname_handles_bad_input() {
        // The function requires the hostname to necessarily be there in the URL,
        // but should fail gracefully if that is not the case.
        // Graceful failure here is returning an empty string for the rest of the URL
        assert_eq!(
            get_url_after_hostname("https://www.google.com/ad", "google.com"),
            "/ad"
        );
        assert_eq!(
            get_url_after_hostname(
                "https://www.google.com/?aclksa=l&ai=DChcSEwioqMfq5",
                "google.com"
            ),
            "/?aclksa=l&ai=DChcSEwioqMfq5"
        );
        assert_eq!(
            get_url_after_hostname(
                "https://www.google.com/?aclksa=l&ai=DChcSEwioqMfq5",
                "www.google.com"
            ),
            "/?aclksa=l&ai=DChcSEwioqMfq5"
        );
        assert_eq!(
            get_url_after_hostname(
                "https://www.youtube.com/?aclksa=l&ai=DChcSEwioqMfq5",
                "google.com"
            ),
            ""
        );
    }
}

#[cfg(test)]
mod hash_collision_tests {
    use super::super::*;

    use crate::lists::parse_filters;
    use crate::test_utils;
    use std::collections::HashMap;

    #[test]
    fn check_rule_ids_no_collisions() {
        let rules = test_utils::rules_from_lists([
            "data/easylist.to/easylist/easylist.txt",
            "data/easylist.to/easylist/easyprivacy.txt",
        ]);
        let (network_filters, _) = parse_filters(rules, true, Default::default());

        let mut filter_ids: HashMap<Hash, String> = HashMap::new();

        for filter in network_filters {
            let id = filter.get_id();
            let rule = *filter.raw_line.unwrap_or_default();
            let existing_rule = filter_ids.get(&id);
            assert!(
                existing_rule.is_none() || existing_rule.unwrap() == &rule,
                "ID {} for {} already present from {}",
                id,
                rule,
                existing_rule.unwrap()
            );
            filter_ids.insert(id, rule);
        }
    }
}
