#[cfg(test)]
mod parse_tests {
    use super::super::*;
    use crate::request::Request;

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
            assert!(filter.is_important());
        }
        {
            // parses ~important
            let filter = NetworkFilter::parse("||foo.com$~important", true, Default::default());
            assert_eq!(filter.err(), Some(NetworkFilterError::NegatedImportant));
        }
        {
            // defaults to false
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert!(!filter.is_important());
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
            assert!(filter.is_csp());
            assert_eq!(filter.modifier_option, Some(String::from(r#"self bar """#)));
        }
        {
            // parses empty CSP
            let filter = NetworkFilter::parse("||foo.com$csp", true, Default::default()).unwrap();
            assert!(filter.is_csp());
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
            assert!(filter.match_case());
        }
        {
            let filter = NetworkFilter::parse(r#"/^https?:\/\/[a-z]{8,15}\.top\/[-a-z]{4,}\.css\?aHR0c[\/0-9a-zA-Z]{33,}=?=?\$/$css,3p,match-case"#, true, Default::default()).unwrap();
            assert!(filter.match_case());
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
            assert!(!filter.match_case())
        }
    }

    #[test]
    fn parses_first_party() {
        // parses first-party
        assert!(
            NetworkFilter::parse("||foo.com$first-party", true, Default::default())
                .unwrap()
                .first_party()
        );
        assert!(
            NetworkFilter::parse("@@||foo.com$first-party", true, Default::default())
                .unwrap()
                .first_party()
        );
        assert!(
            NetworkFilter::parse("@@||foo.com|$first-party", true, Default::default())
                .unwrap()
                .first_party()
        );
        // parses ~first-party
        assert!(
            !NetworkFilter::parse("||foo.com$~first-party", true, Default::default())
                .unwrap()
                .first_party()
        );
        assert!(!NetworkFilter::parse(
            "||foo.com$first-party,~first-party",
            true,
            Default::default()
        )
        .unwrap()
        .first_party());
        // defaults to true
        assert!(NetworkFilter::parse("||foo.com", true, Default::default())
            .unwrap()
            .first_party());
    }

    #[test]
    fn parses_third_party() {
        // parses third-party
        assert!(
            NetworkFilter::parse("||foo.com$third-party", true, Default::default())
                .unwrap()
                .third_party()
        );
        assert!(
            NetworkFilter::parse("@@||foo.com$third-party", true, Default::default())
                .unwrap()
                .third_party()
        );
        assert!(
            NetworkFilter::parse("@@||foo.com|$third-party", true, Default::default())
                .unwrap()
                .third_party()
        );
        assert!(
            NetworkFilter::parse("||foo.com$~first-party", true, Default::default())
                .unwrap()
                .third_party()
        );
        // parses ~third-party
        assert!(
            !NetworkFilter::parse("||foo.com$~third-party", true, Default::default())
                .unwrap()
                .third_party()
        );
        assert!(!NetworkFilter::parse(
            "||foo.com$first-party,~third-party",
            true,
            Default::default()
        )
        .unwrap()
        .third_party());
        // defaults to true
        assert!(NetworkFilter::parse("||foo.com", true, Default::default())
            .unwrap()
            .third_party());
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
            assert!(filter.is_exception());
            assert!(filter.is_generic_hide());
        }
        {
            let filter =
                NetworkFilter::parse("@@||foo.com|$generichide", true, Default::default()).unwrap();
            assert!(filter.is_exception());
            assert!(filter.is_generic_hide());
        }
        {
            let filter = NetworkFilter::parse(
                "@@$generichide,domain=example.com",
                true,
                Default::default(),
            )
            .unwrap();
            assert!(filter.is_generic_hide());
            let breakdown = NetworkFilterBreakdown::from(&filter);
            assert_eq!(
                breakdown.opt_domains,
                Some(vec![utils::fast_hash("example.com")])
            );
        }
        {
            let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
            assert!(!filter.is_generic_hide());
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
                NetworkFilter::parse(&format!("||foo.com${option}"), true, Default::default());
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
                let filter =
                    NetworkFilter::parse(&format!("||foo.com${option}"), true, Default::default())
                        .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, false);
                set_option(option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com$object,{option}"),
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
                set_option(option, &mut defaults, true);
                set_option("object", &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com$domain=bar.com,{option}"),
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
                set_option(option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            // negative
            {
                let filter =
                    NetworkFilter::parse(&format!("||foo.com$~{option}"), true, Default::default())
                        .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_network_types = false;
                set_all_options(&mut defaults, true);
                set_option(option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter = NetworkFilter::parse(
                    &format!("||foo.com${option},~{option}"),
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
                set_option(option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }
            // default - positive
            {
                let filter = NetworkFilter::parse("||foo.com", true, Default::default()).unwrap();
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

    #[test]
    fn test_simple_pattern_tokenization() {
        let rule = "||primewire.*/sw$script,1p";
        let filter =
            NetworkFilter::parse(rule, true, crate::lists::ParseOptions::default()).unwrap();
        let tokens = filter.get_tokens_optimized();
        assert_eq!(
            tokens,
            crate::filters::network::FilterTokens::Other(vec![utils::fast_hash("primewire")])
        );
    }

    #[test]
    fn test_no_filters_return_empty_tokens() {
        // Test to ensure no filters return FilterTokens::Empty after tokenization fixes
        // Empty tokens cause filters to be completely skipped during indexing

        // Test various filter types to ensure they all get proper tokens
        let test_rules = [
            // Hostname regex filters (the main fix)
            "||adservice.google.*/adsid/integrator.js$xhr",
            "||primewire.*/sw$script,1p",
            "||google-analytics.com*^$script",

            // Regular hostname filters
            "||example.com^",
            "||test.com/path",

            // Domain-restricted filters
            "||google.com^$domain=example.com",

            // Plain patterns
            "/ads/tracking.js",
            "||doubleclick.net^",

            // Complex patterns
            "*://*google*com/ads/*",
        ];

        for rule in &test_rules {
            let filter = NetworkFilter::parse(rule, true, crate::lists::ParseOptions::default())
                .unwrap_or_else(|_| panic!("Failed to parse rule: {}", rule));

            let tokens = filter.get_tokens_optimized();
            match tokens {
                crate::filters::network::FilterTokens::Empty => {
                    panic!("Rule '{}' returns FilterTokens::Empty - will be skipped during indexing!", rule);
                }
                crate::filters::network::FilterTokens::OptDomains(domains) => {
                    assert!(!domains.is_empty(), "Rule '{}' has empty OptDomains", rule);
                }
                crate::filters::network::FilterTokens::Other(_token_vec) => {
                    // Other tokens are fine, even if empty vector (gets processed)
                }
            }
        }
    }

    #[test]
    fn test_scan_brave_list_for_empty_tokens() {
        // Comprehensive test: scan the actual brave-main-list.txt for any filters
        // that return FilterTokens::Empty (which would be skipped during indexing)
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open("data/brave/brave-main-list.txt")
            .expect("Failed to open brave-main-list.txt");
        let reader = BufReader::new(file);

        let mut empty_token_rules = Vec::new();
        let mut opt_domains_count = 0;
        let mut other_tokens_count = 0;
        let mut total_rules = 0;

        for line_result in reader.lines() {
            let line = line_result.expect("Failed to read line").trim().to_string();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('!') || line.starts_with('[') {
                continue;
            }

            total_rules += 1;

            // Try to parse as network filter
            if let Ok(filter) = NetworkFilter::parse(&line, true, crate::lists::ParseOptions::default()) {
                let tokens = filter.get_tokens_optimized();
                match tokens {
                    crate::filters::network::FilterTokens::Empty => {
                        empty_token_rules.push(line);
                    }
                    crate::filters::network::FilterTokens::OptDomains(_) => {
                        opt_domains_count += 1;
                    }
                    crate::filters::network::FilterTokens::Other(_) => {
                        other_tokens_count += 1;
                    }
                }
            }
        }

        println!("Scanned {} rules from brave-main-list.txt", total_rules);
        println!("Token distribution:");
        println!("  Empty tokens: {}", empty_token_rules.len());
        println!("  OptDomains: {}", opt_domains_count);
        println!("  Other tokens: {}", other_tokens_count);

        if !empty_token_rules.is_empty() {
            println!("Rules with Empty tokens:");
            for rule in empty_token_rules.iter().take(5) {
                println!("  '{}'", rule);
            }
            if empty_token_rules.len() > 5 {
                println!("  ... and {} more", empty_token_rules.len() - 5);
            }
            panic!("Found {} rules that return FilterTokens::Empty and will be skipped!", empty_token_rules.len());
        }

        // This test should pass if no rules return Empty tokens
        assert_eq!(empty_token_rules.len(), 0, "No rules should return FilterTokens::Empty");
    }

    #[test]
    fn test_empty_tokens_behavior() {
        // Test what happens to filters with FilterTokens::Empty
        // This is important for understanding if such filters are handled correctly

        // Create a filter that might have empty tokens
        // (This is hard to do with the current tokenization logic, but let's test the behavior)

        use crate::Engine;

        // Create an engine with some basic rules
        let rules = vec![
            "||example.com^".to_string(),  // Should have tokens
            "/ads.js".to_string(),         // Should have tokens
        ];

        let engine = Engine::from_rules(rules, Default::default());

        // Test a request that should be blocked
        let request = Request::new(
            "https://example.com/ads.js",
            "https://example.com",
            "script"
        ).unwrap();

        let result = engine.check_network_request(&request);
        assert!(result.matched, "Request to example.com/ads.js should be blocked");

        // Now let's think about what would happen if we had a filter with Empty tokens:
        // 1. It wouldn't be indexed in any filter map
        // 2. It would never be checked against any requests
        // 3. It would be effectively dead code

        // This is actually a problem! If a filter legitimately should match but has no tokens,
        // it would never be evaluated.

        // The question is: are there any legitimate cases where a filter should have Empty tokens?
        // Answer: Probably not in a well-designed system. All filters should have some way to
        // be indexed and checked.

        // Our comprehensive test above shows that no filters in the brave-main-list.txt
        // have Empty tokens, which is good - it means all filters are properly tokenized.
    }
}
