#[cfg(test)]
mod match_tests {
    use super::super::*;
    use crate::filters::network::*;

    #[test]
    fn is_anchored_by_hostname_works() {
        // matches empty hostname
        assert!(is_anchored_by_hostname("", "foo.com", false));

        // does not match when filter hostname is longer than hostname
        assert!(
            !is_anchored_by_hostname("bar.foo.com", "foo.com", false)
        );
        assert!(!is_anchored_by_hostname("b", "", false));
        assert!(!is_anchored_by_hostname("foo.com", "foo.co", false));

        // does not match if there is not match
        assert!(!is_anchored_by_hostname("bar", "foo.com", false));

        // ## prefix match
        // matches exact match
        assert!(is_anchored_by_hostname("", "", false));
        assert!(is_anchored_by_hostname("f", "f", false));
        assert!(is_anchored_by_hostname("foo", "foo", false));
        assert!(is_anchored_by_hostname("foo.com", "foo.com", false));
        assert!(is_anchored_by_hostname(".com", ".com", false));
        assert!(is_anchored_by_hostname("com.", "com.", false));

        // matches partial
        // Single label
        assert!(is_anchored_by_hostname("foo", "foo.com", false));
        assert!(is_anchored_by_hostname("foo.", "foo.com", false));
        assert!(is_anchored_by_hostname(".foo", ".foo.com", false));
        assert!(is_anchored_by_hostname(".foo.", ".foo.com", false));

        // Multiple labels
        assert!(is_anchored_by_hostname("foo.com", "foo.com.", false));
        assert!(is_anchored_by_hostname("foo.com.", "foo.com.", false));
        assert!(
            is_anchored_by_hostname(".foo.com.", ".foo.com.", false)
        );
        assert!(is_anchored_by_hostname(".foo.com", ".foo.com", false));

        assert!(
            is_anchored_by_hostname("foo.bar", "foo.bar.com", false)
        );
        assert!(
            is_anchored_by_hostname("foo.bar.", "foo.bar.com", false)
        );

        // does not match partial prefix
        // Single label
        assert!(!is_anchored_by_hostname("foo", "foobar.com", false));
        assert!(!is_anchored_by_hostname("fo", "foo.com", false));
        assert!(!is_anchored_by_hostname(".foo", "foobar.com", false));

        // Multiple labels
        assert!(
            !is_anchored_by_hostname("foo.bar", "foo.barbaz.com", false)
        );
        assert!(
            !is_anchored_by_hostname(".foo.bar", ".foo.barbaz.com", false)
        );

        // ## suffix match
        // matches partial
        // Single label
        assert!(is_anchored_by_hostname("com", "foo.com", false));
        assert!(is_anchored_by_hostname(".com", "foo.com", false));
        assert!(is_anchored_by_hostname(".com.", "foo.com.", false));
        assert!(is_anchored_by_hostname("com.", "foo.com.", false));

        // Multiple labels
        assert!(
            is_anchored_by_hostname("foo.com.", ".foo.com.", false)
        );
        assert!(is_anchored_by_hostname("foo.com", ".foo.com", false));

        // does not match partial
        // Single label
        assert!(!is_anchored_by_hostname("om", "foo.com", false));
        assert!(!is_anchored_by_hostname("com", "foocom", false));

        // Multiple labels
        assert!(
            !is_anchored_by_hostname("foo.bar.com", "baz.bar.com", false)
        );
        assert!(
            !is_anchored_by_hostname("fo.bar.com", "foo.bar.com", false)
        );
        assert!(
            !is_anchored_by_hostname(".fo.bar.com", "foo.bar.com", false)
        );
        assert!(
            !is_anchored_by_hostname("bar.com", "foobar.com", false)
        );
        assert!(
            !is_anchored_by_hostname(".bar.com", "foobar.com", false)
        );

        // ## infix match
        // matches partial
        assert!(is_anchored_by_hostname("bar", "foo.bar.com", false));
        assert!(is_anchored_by_hostname("bar.", "foo.bar.com", false));
        assert!(is_anchored_by_hostname(".bar.", "foo.bar.com", false));
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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

    fn check_options(filter: &NetworkFilter, request: &request::Request) -> bool {
        super::super::check_options(filter.mask, request)
            && super::super::check_included_domains(filter.opt_domains.as_deref(), request)
            && super::super::check_excluded_domains(filter.opt_not_domains.as_deref(), request)
    }

    #[test]
    // options
    fn check_options_works() {
        // cpt test
        {
            let network_filter =
                NetworkFilter::parse("||foo$image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "image").unwrap();
            assert!(check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "script").unwrap();
            assert!(!check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~image", true, Default::default()).unwrap();
            let request = request::Request::new("https://foo.com/bar", "", "script").unwrap();
            assert!(check_options(&network_filter, &request));
        }

        // ~third-party
        {
            let network_filter =
                NetworkFilter::parse("||foo$~third-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.foo.com", "").unwrap();
            assert!(check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~third-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.bar.com", "").unwrap();
            assert!(!check_options(&network_filter, &request));
        }

        // ~first-party
        {
            let network_filter =
                NetworkFilter::parse("||foo$~first-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.bar.com", "").unwrap();
            assert!(check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$~first-party", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://baz.foo.com", "").unwrap();
            assert!(!check_options(&network_filter, &request));
        }

        // opt-domain
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=foo.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert!(check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=foo.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert!(!check_options(&network_filter, &request));
        }

        // opt-not-domain
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=~bar.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert!(check_options(&network_filter, &request));
        }
        {
            let network_filter =
                NetworkFilter::parse("||foo$domain=~bar.com", true, Default::default()).unwrap();
            let request =
                request::Request::new("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert!(!check_options(&network_filter, &request));
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
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                )
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
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                )
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
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                )
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
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://subfoo.foo.example.com",
                        ""
                    )
                    .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.example.com", "")
                        .unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new(
                        "http://example.net/adv",
                        "http://anotherexample.com",
                        ""
                    )
                    .unwrap()
                )
            );
        }
        {
            let network_filter =
                NetworkFilter::parse("adv$domain=com|~foo.com", true, Default::default()).unwrap();
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://com", "").unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://foo.com", "").unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://subfoo.foo.com", "")
                        .unwrap()
                )
            );
            assert!(
                network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://bar.com", "").unwrap()
                )
            );
            assert!(
                !network_filter.matches_test(
                    &request::Request::new("http://example.net/adv", "http://co.uk", "").unwrap()
                )
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
                network_filter.matches_test(&request),
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
