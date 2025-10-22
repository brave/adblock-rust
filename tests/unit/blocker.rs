#[cfg(test)]
mod blocker_tests {

    use super::super::*;
    use crate::lists::parse_filters;
    use crate::request::Request;
    use crate::resources::{Resource, ResourceStorage};
    use base64::{engine::Engine as _, prelude::BASE64_STANDARD};
    use std::collections::HashSet;
    use std::iter::FromIterator;

    #[test]
    fn single_slash() {
        let filters = ["/|"];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: true,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        let request = Request::new(
            "https://example.com/test/",
            "https://example.com",
            "xmlhttprequest",
        )
        .unwrap();
        assert!(blocker.check(&request, &Default::default()).matched);

        let request = Request::new(
            "https://example.com/test",
            "https://example.com",
            "xmlhttprequest",
        )
        .unwrap();
        assert!(!blocker.check(&request, &Default::default()).matched);
    }

    fn test_requests_filters(
        filters: impl IntoIterator<Item = impl AsRef<str>>,
        requests: &[(Request, bool)],
    ) {
        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        requests.iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(req, &Default::default());
            if *expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(
                    !matched_rule.matched,
                    "Expected no match for {}, matched with {:?}",
                    req.url, matched_rule.filter
                );
            }
        });
    }

    #[test]
    fn redirect_blocking_exception() {
        let filters = [
            "||imdb-video.media-imdb.com$media,redirect=noop-0.1s.mp3",
            "@@||imdb-video.media-imdb.com^$domain=imdb.com",
        ];

        let request = Request::new(
            "https://imdb-video.media-imdb.com/kBOeI88k1o23eNAi",
            "https://www.imdb.com/video/13",
            "media",
        )
        .unwrap();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::in_memory_from_resources([Resource::simple(
            "noop-0.1s.mp3",
            crate::resources::MimeType::AudioMp3,
            "mp3",
        )]);

        let matched_rule = blocker.check(&request, &resources);
        assert!(!matched_rule.matched);
        assert!(!matched_rule.important);
        assert_eq!(
            matched_rule.redirect,
            Some("data:audio/mp3;base64,bXAz".to_string())
        );
        assert_eq!(
            matched_rule.exception,
            Some("@@||imdb-video.media-imdb.com^$domain=imdb.com".to_string())
        );
    }

    #[test]
    fn redirect_exception() {
        let filters = [
            "||imdb-video.media-imdb.com$media,redirect=noop-0.1s.mp3",
            "@@||imdb-video.media-imdb.com^$domain=imdb.com,redirect=noop-0.1s.mp3",
        ];

        let request = Request::new(
            "https://imdb-video.media-imdb.com/kBOeI88k1o23eNAi",
            "https://www.imdb.com/video/13",
            "media",
        )
        .unwrap();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::in_memory_from_resources([Resource::simple(
            "noop-0.1s.mp3",
            crate::resources::MimeType::AudioMp3,
            "mp3",
        )]);

        let matched_rule = blocker.check(&request, &resources);
        assert!(!matched_rule.matched);
        assert!(!matched_rule.important);
        assert_eq!(matched_rule.redirect, None);
        assert_eq!(
            matched_rule.exception,
            Some(
                "@@||imdb-video.media-imdb.com^$domain=imdb.com,redirect=noop-0.1s.mp3".to_string()
            )
        );
    }

    #[test]
    fn redirect_rule_redirection() {
        let filters = [
            "||doubleclick.net^",
            "||www3.doubleclick.net^$xmlhttprequest,redirect-rule=noop.txt,domain=lineups.fun",
        ];

        let request =
            Request::new("https://www3.doubleclick.net", "https://lineups.fun", "xhr").unwrap();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::in_memory_from_resources([Resource::simple(
            "noop.txt",
            crate::resources::MimeType::TextPlain,
            "noop",
        )]);

        let matched_rule = blocker.check(&request, &resources);
        assert!(matched_rule.matched);
        assert!(!matched_rule.important);
        assert_eq!(
            matched_rule.redirect,
            Some("data:text/plain;base64,bm9vcA==".to_string())
        );
        assert_eq!(matched_rule.exception, None);
    }

    #[test]
    fn badfilter_does_not_match() {
        let filters = ["||foo.com$badfilter"];
        let url_results = [(
            Request::new("https://foo.com", "https://bar.com", "image").unwrap(),
            false,
        )];

        let request_expectations: Vec<_> = url_results.into_iter().collect();

        test_requests_filters(filters, &request_expectations);
    }

    #[test]
    fn badfilter_cancels_with_same_id() {
        let filters = [
            "||foo.com$domain=bar.com|foo.com,badfilter",
            "||foo.com$domain=foo.com|bar.com",
        ];
        let url_results = [(
            Request::new("https://foo.com", "https://bar.com", "image").unwrap(),
            false,
        )];

        let request_expectations: Vec<_> = url_results.into_iter().collect();

        test_requests_filters(filters, &request_expectations);
    }

    #[test]
    fn badfilter_does_not_cancel_similar_filter() {
        let filters = [
            "||foo.com$domain=bar.com|foo.com,badfilter",
            "||foo.com$domain=foo.com|bar.com,image",
        ];
        let url_results = [(
            Request::new("https://foo.com", "https://bar.com", "image").unwrap(),
            true,
        )];

        let request_expectations: Vec<_> = url_results.into_iter().collect();

        test_requests_filters(filters, &request_expectations);
    }

    #[test]
    fn hostname_regex_filter_works() {
        let filters = [
            "||alimc*.top^$domain=letv.com",
            "||aa*.top^$domain=letv.com",
        ];
        let url_results = [
            (
                Request::new(
                    "https://r.alimc1.top/test.js",
                    "https://minisite.letv.com/",
                    "script",
                )
                .unwrap(),
                true,
            ),
            (
                Request::new(
                    "https://www.baidu.com/test.js",
                    "https://minisite.letv.com/",
                    "script",
                )
                .unwrap(),
                false,
            ),
            (
                Request::new(
                    "https://r.aabb.top/test.js",
                    "https://example.com/",
                    "script",
                )
                .unwrap(),
                false,
            ),
            (
                Request::new(
                    "https://r.aabb.top/test.js",
                    "https://minisite.letv.com/",
                    "script",
                )
                .unwrap(),
                true,
            ),
        ];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::default();

        url_results.into_iter().for_each(|(req, expected_result)| {
            let matched_rule = blocker.check(&req, &resources);
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", req.url);
            } else {
                assert!(
                    !matched_rule.matched,
                    "Expected no match for {}, matched with {:?}",
                    req.url, matched_rule.filter
                );
            }
        });
    }

    #[test]
    fn get_csp_directives() {
        let filters = [
            "$csp=script-src 'self' * 'unsafe-inline',domain=thepiratebay.vip|pirateproxy.live|thehiddenbay.com|downloadpirate.com|thepiratebay10.org|kickass.vip|pirateproxy.app|ukpass.co|prox.icu|pirateproxy.life",
            "$csp=worker-src 'none',domain=pirateproxy.live|thehiddenbay.com|tpb.party|thepiratebay.org|thepiratebay.vip|thepiratebay10.org|flashx.cc|vidoza.co|vidoza.net",
            "||1337x.to^$csp=script-src 'self' 'unsafe-inline'",
            "@@^no-csp^$csp=script-src 'self' 'unsafe-inline'",
            "^duplicated-directive^$csp=worker-src 'none'",
            "@@^disable-all^$csp",
            "^first-party-only^$csp=script-src 'none',1p",
        ];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: false,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        {
            // No directives should be returned for requests that are not `document` or `subdocument` content types.
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://pirateproxy.live/static/custom_ads.js",
                        "https://pirateproxy.live",
                        "script"
                    )
                    .unwrap()
                ),
                None
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://pirateproxy.live/static/custom_ads.js",
                        "https://pirateproxy.live",
                        "image"
                    )
                    .unwrap()
                ),
                None
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://pirateproxy.live/static/custom_ads.js",
                        "https://pirateproxy.live",
                        "object"
                    )
                    .unwrap()
                ),
                None
            );
        }
        {
            // A single directive should be returned if only one match is present in the engine, for both document and subdocument types
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new("https://example.com", "https://vidoza.co", "document").unwrap()
                ),
                Some(String::from("worker-src 'none'"))
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new("https://example.com", "https://vidoza.net", "subdocument")
                        .unwrap()
                ),
                Some(String::from("worker-src 'none'"))
            );
        }
        {
            // Multiple merged directives should be returned if more than one match is present in the engine
            let possible_results = [
                Some(String::from(
                    "script-src 'self' * 'unsafe-inline',worker-src 'none'",
                )),
                Some(String::from(
                    "worker-src 'none',script-src 'self' * 'unsafe-inline'",
                )),
            ];
            assert!(possible_results.contains(
                &blocker.get_csp_directives(
                    &Request::new(
                        "https://example.com",
                        "https://pirateproxy.live",
                        "document"
                    )
                    .unwrap()
                )
            ));
            assert!(possible_results.contains(
                &blocker.get_csp_directives(
                    &Request::new(
                        "https://example.com",
                        "https://pirateproxy.live",
                        "subdocument"
                    )
                    .unwrap()
                )
            ));
        }
        {
            // A directive with an exception should not be returned
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new("https://1337x.to", "https://1337x.to", "document").unwrap()
                ),
                Some(String::from("script-src 'self' 'unsafe-inline'"))
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new("https://1337x.to/no-csp", "https://1337x.to", "subdocument")
                        .unwrap()
                ),
                None
            );
        }
        {
            // Multiple identical directives should only appear in the output once
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://example.com/duplicated-directive",
                        "https://flashx.cc",
                        "document"
                    )
                    .unwrap()
                ),
                Some(String::from("worker-src 'none'"))
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://example.com/duplicated-directive",
                        "https://flashx.cc",
                        "subdocument"
                    )
                    .unwrap()
                ),
                Some(String::from("worker-src 'none'"))
            );
        }
        {
            // A CSP exception with no corresponding directive should disable all CSP injections for the page
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://1337x.to/duplicated-directive/disable-all",
                        "https://thepiratebay10.org",
                        "document"
                    )
                    .unwrap()
                ),
                None
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://1337x.to/duplicated-directive/disable-all",
                        "https://thepiratebay10.org",
                        "document"
                    )
                    .unwrap()
                ),
                None
            );
        }
        {
            // A CSP exception with a partyness modifier should only match where the modifier applies
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "htps://github.com/first-party-only",
                        "https://example.com",
                        "subdocument"
                    )
                    .unwrap()
                ),
                None
            );
            assert_eq!(
                blocker.get_csp_directives(
                    &Request::new(
                        "https://example.com/first-party-only",
                        "https://example.com",
                        "document"
                    )
                    .unwrap()
                ),
                Some(String::from("script-src 'none'"))
            );
        }
    }

    #[test]
    fn test_removeparam() {
        let filters = [
            "||example.com^$removeparam=test",
            "*$removeparam=fbclid",
            "/script.js$redirect-rule=noopjs",
            "^block^$important",
            "$removeparam=testCase,~xhr",
        ];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: true,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::in_memory_from_resources([Resource::simple(
            "noopjs",
            crate::resources::MimeType::ApplicationJavascript,
            "(() => {})()",
        )]);

        let result = blocker.check(
            &Request::new(
                "https://example.com?q=1&test=2#blue",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?q=1#blue".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?test=2&q=1#blue",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?q=1#blue".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?test=2#blue",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com#blue".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new("https://example.com?q=1#blue", "https://antonok.com", "xhr").unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?q=1&test=2",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, Some("https://example.com?q=1".into()));
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?test=2&q=1",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, Some("https://example.com?q=1".into()));
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new("https://example.com?test=2", "https://antonok.com", "xhr").unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, Some("https://example.com".into()));
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new("https://example.com?test=2", "https://antonok.com", "image").unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new("https://example.com?q=1", "https://antonok.com", "xhr").unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new("https://example.com?q=fbclid", "https://antonok.com", "xhr").unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?fbclid=10938&q=1&test=2",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, Some("https://example.com?q=1".into()));
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://test.com?fbclid=10938&q=1&test=2",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://test.com?q=1&test=2".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?q1=1&q2=2&q3=3&test=2&q4=4&q5=5&fbclid=39",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?q1=1&q2=2&q3=3&q4=4&q5=5".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?q1=1&q1=2&test=2&test=3",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?q1=1&q1=2".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/script.js?test=2#blue",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com/script.js#blue".into())
        );
        assert_eq!(
            result.redirect,
            Some("data:application/javascript;base64,KCgpID0+IHt9KSgp".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/block/script.js?test=2",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert_eq!(
            result.redirect,
            Some("data:application/javascript;base64,KCgpID0+IHt9KSgp".into())
        );
        assert!(result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/Path/?Test=ABC&testcase=AbC&testCase=aBc",
                "https://antonok.com",
                "xhr",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/Path/?Test=ABC&testcase=AbC&testCase=aBc",
                "https://antonok.com",
                "image",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/Path/?Test=ABC&testcase=AbC&testCase=aBc",
                "https://antonok.com",
                "subdocument",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com/Path/?Test=ABC&testcase=AbC".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/Path/?Test=ABC&testcase=AbC&testCase=aBc",
                "https://antonok.com",
                "document",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com/Path/?Test=ABC&testcase=AbC".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?Test=ABC?123&test=3#&test=4#b",
                "https://antonok.com",
                "document",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?Test=ABC?123#&test=4#b".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?Test=ABC&testCase=5",
                "https://antonok.com",
                "document",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(
            result.rewritten_url,
            Some("https://example.com?Test=ABC".into())
        );
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com?Test=ABC&testCase=5",
                "https://antonok.com",
                "image",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.rewritten_url, None);
        assert!(!result.matched);
    }

    /// Tests ported from the previous query parameter stripping logic in brave-core
    #[test]
    fn removeparam_brave_core_tests() {
        let testcases = [
            // (original url, expected url after filtering)
            ("https://example.com/?fbclid=1234", "https://example.com/"),
            ("https://example.com/?fbclid=1234&", "https://example.com/"),
            ("https://example.com/?&fbclid=1234", "https://example.com/"),
            ("https://example.com/?gclid=1234", "https://example.com/"),
            (
                "https://example.com/?fbclid=0&gclid=1&msclkid=a&mc_eid=a1",
                "https://example.com/",
            ),
            (
                "https://example.com/?fbclid=&foo=1&bar=2&gclid=abc",
                "https://example.com/?fbclid=&foo=1&bar=2",
            ),
            (
                "https://example.com/?fbclid=&foo=1&gclid=1234&bar=2",
                "https://example.com/?fbclid=&foo=1&bar=2",
            ),
            (
                "http://u:p@example.com/path/file.html?foo=1&fbclid=abcd#fragment",
                "http://u:p@example.com/path/file.html?foo=1#fragment",
            ),
            ("https://example.com/?__s=1234-abcd", "https://example.com/"),
            // Obscure edge cases that break most parsers:
            (
                "https://example.com/?fbclid&foo&&gclid=2&bar=&%20",
                "https://example.com/?fbclid&foo&&bar=&%20",
            ),
            (
                "https://example.com/?fbclid=1&1==2&=msclkid&foo=bar&&a=b=c&",
                "https://example.com/?1==2&=msclkid&foo=bar&&a=b=c&",
            ),
            (
                "https://example.com/?fbclid=1&=2&?foo=yes&bar=2+",
                "https://example.com/?=2&?foo=yes&bar=2+",
            ),
            (
                "https://example.com/?fbclid=1&a+b+c=some%20thing&1%202=3+4",
                "https://example.com/?a+b+c=some%20thing&1%202=3+4",
            ),
            // Conditional query parameter stripping
            /*("https://example.com/?mkt_tok=123&foo=bar",
            "https://example.com/?foo=bar"),*/
        ];

        let filters = [
            "fbclid",
            "gclid",
            "msclkid",
            "mc_eid",
            "dclid",
            "oly_anon_id",
            "oly_enc_id",
            "_openstat",
            "vero_conv",
            "vero_id",
            "wickedid",
            "yclid",
            "__s",
            "rb_clickid",
            "s_cid",
            "ml_subscriber",
            "ml_subscriber_hash",
            "twclid",
            "gbraid",
            "wbraid",
            "_hsenc",
            "__hssc",
            "__hstc",
            "__hsfp",
            "hsCtaTracking",
            "oft_id",
            "oft_k",
            "oft_lk",
            "oft_d",
            "oft_c",
            "oft_ck",
            "oft_ids",
            "oft_sk",
            "ss_email_id",
            "bsft_uid",
            "bsft_clkid",
            "vgo_ee",
            "igshid",
        ]
        .iter()
        .map(|s| format!("*$removeparam={}", s))
        .collect::<Vec<_>>();

        let (network_filters, _) = parse_filters(&filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: true,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        let resources = ResourceStorage::default();

        for (original, expected) in testcases.into_iter() {
            let result = blocker.check(
                &Request::new(original, "https://example.net", "xhr").unwrap(),
                &resources,
            );
            let expected = if original == expected {
                None
            } else {
                Some(expected.to_string())
            };
            assert_eq!(
                expected, result.rewritten_url,
                "Filtering parameters on {} failed",
                original
            );
        }
    }

    #[test]
    fn test_removeparam_same_tokens() {
        let filters = ["$removeparam=example1_", "$removeparam=example1-"];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: true,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        let result = blocker.check(
            &Request::new(
                "https://example.com?example1_=1&example1-=2",
                "https://example.com",
                "xhr",
            )
            .unwrap(),
            &Default::default(),
        );
        assert_eq!(result.rewritten_url, Some("https://example.com".into()));
        assert!(!result.matched);
    }

    #[test]
    fn test_redirect_priority() {
        let filters = [
            ".txt^$redirect-rule=a",
            "||example.com^$redirect-rule=b:10",
            "/text$redirect-rule=c:20",
            "@@^excepta^$redirect-rule=a",
            "@@^exceptb10^$redirect-rule=b:10",
            "@@^exceptc20^$redirect-rule=c:20",
        ];

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options = BlockerOptions {
            enable_optimizations: true,
        };

        let blocker = Blocker::new(network_filters, &blocker_options);
        fn simple_resource(identifier: &str) -> Resource {
            Resource::simple(
                identifier,
                crate::resources::MimeType::TextPlain,
                identifier,
            )
        }
        fn simple_redirect(identifier: &str) -> String {
            format!(
                "data:text/plain;base64,{}",
                BASE64_STANDARD.encode(identifier)
            )
        }
        let test_cases = ["a", "b", "c"];
        let resources = ResourceStorage::in_memory_from_resources(test_cases.map(simple_resource));
        let redirects = test_cases
            .into_iter()
            .map(simple_redirect)
            .collect::<Vec<_>>();
        let a_redirect = Some(redirects[0].clone());
        let b_redirect = Some(redirects[1].clone());
        let c_redirect = Some(redirects[2].clone());

        let result = blocker.check(
            &Request::new(
                "https://example.net/test",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.net/test.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, a_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/test.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, b_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/text.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, c_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/exceptc20/text.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, b_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/exceptb10/text.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, c_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/exceptc20/exceptb10/text.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, a_redirect);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/exceptc20/exceptb10/excepta/text.txt",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, None);
        assert!(!result.matched);

        let result = blocker.check(
            &Request::new(
                "https://example.com/exceptc20/exceptb10/text",
                "https://example.com",
                "xmlhttprequest",
            )
            .unwrap(),
            &resources,
        );
        assert_eq!(result.redirect, None);
        assert!(!result.matched);
    }

    #[test]
    fn tags_enable_works() {
        let filters = [
            "adv$tag=stuff",
            "somelongpath/test$tag=stuff",
            "||brianbondy.com/$tag=brian",
            "||brave.com$tag=brian",
        ];
        let url_results = [
            ("http://example.com/advert.html", true),
            ("http://example.com/somelongpath/test/2.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brave.com/about", false),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        let resources = Default::default();
        blocker.enable_tags(&["stuff"]);
        assert_eq!(
            blocker.tags_enabled,
            HashSet::from_iter([String::from("stuff")].into_iter())
        );

        request_expectations
            .into_iter()
            .for_each(|(req, expected_result)| {
                let matched_rule = blocker.check(&req, &resources);
                if expected_result {
                    assert!(matched_rule.matched, "Expected match for {}", req.url);
                } else {
                    assert!(
                        !matched_rule.matched,
                        "Expected no match for {}, matched with {:?}",
                        req.url, matched_rule.filter
                    );
                }
            });
    }

    #[test]
    fn tags_enable_adds_tags() {
        let filters = [
            "adv$tag=stuff",
            "somelongpath/test$tag=stuff",
            "||brianbondy.com/$tag=brian",
            "||brave.com$tag=brian",
        ];
        let url_results = [
            ("http://example.com/advert.html", true),
            ("http://example.com/somelongpath/test/2.html", true),
            ("https://brianbondy.com/about", true),
            ("https://brave.com/about", true),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        let resources = Default::default();
        blocker.enable_tags(&["stuff"]);
        blocker.enable_tags(&["brian"]);
        assert_eq!(
            blocker.tags_enabled,
            HashSet::from_iter([String::from("brian"), String::from("stuff")].into_iter())
        );

        request_expectations
            .into_iter()
            .for_each(|(req, expected_result)| {
                let matched_rule = blocker.check(&req, &resources);
                if expected_result {
                    assert!(matched_rule.matched, "Expected match for {}", req.url);
                } else {
                    assert!(
                        !matched_rule.matched,
                        "Expected no match for {}, matched with {:?}",
                        req.url, matched_rule.filter
                    );
                }
            });
    }

    #[test]
    fn tags_disable_works() {
        let filters = [
            "adv$tag=stuff",
            "somelongpath/test$tag=stuff",
            "||brianbondy.com/$tag=brian",
            "||brave.com$tag=brian",
        ];
        let url_results = [
            ("http://example.com/advert.html", false),
            ("http://example.com/somelongpath/test/2.html", false),
            ("https://brianbondy.com/about", true),
            ("https://brave.com/about", true),
        ];

        let request_expectations: Vec<_> = url_results
            .into_iter()
            .map(|(url, expected_result)| {
                let request = Request::new(url, "https://example.com", "other").unwrap();
                (request, expected_result)
            })
            .collect();

        let (network_filters, _) = parse_filters(filters, true, Default::default());

        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let mut blocker = Blocker::new(network_filters, &blocker_options);
        let resources = Default::default();
        blocker.enable_tags(&["brian", "stuff"]);
        assert_eq!(
            blocker.tags_enabled,
            HashSet::from_iter([String::from("brian"), String::from("stuff")].into_iter())
        );
        blocker.disable_tags(&["stuff"]);
        assert_eq!(
            blocker.tags_enabled,
            HashSet::from_iter([String::from("brian")].into_iter())
        );

        request_expectations
            .into_iter()
            .for_each(|(req, expected_result)| {
                let matched_rule = blocker.check(&req, &resources);
                if expected_result {
                    assert!(matched_rule.matched, "Expected match for {}", req.url);
                } else {
                    assert!(
                        !matched_rule.matched,
                        "Expected no match for {}, matched with {:?}",
                        req.url, matched_rule.filter
                    );
                }
            });
    }

    #[test]
    fn exception_force_check() {
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: true,
        };

        let mut filter_set = crate::lists::FilterSet::new(true);
        filter_set
            .add_filter("@@*ad_banner.png", Default::default())
            .unwrap();

        let blocker = Blocker::new(filter_set.network_filters, &blocker_options);

        let resources = Default::default();

        let request = Request::new(
            "http://example.com/ad_banner.png",
            "https://example.com",
            "other",
        )
        .unwrap();

        let matched_rule = blocker.check_parameterised(&request, &resources, false, true);
        assert!(!matched_rule.matched);
        assert!(matched_rule.exception.is_some());
    }

    #[test]
    fn generichide() {
        let blocker_options: BlockerOptions = BlockerOptions {
            enable_optimizations: true,
        };

        let mut filter_set = crate::lists::FilterSet::new(true);
        filter_set
            .add_filter("@@||example.com$generichide", Default::default())
            .unwrap();

        let blocker = Blocker::new(filter_set.network_filters, &blocker_options);

        assert!(blocker.check_generic_hide(
            &Request::new("https://example.com", "https://example.com", "other").unwrap()
        ));
    }
}

#[cfg(test)]
mod placeholder_string_tests {
    /// If this changes, be sure to update the documentation for [`BlockerResult`] as well.
    #[test]
    fn test_constant_placeholder_string() {
        let mut filter_set = crate::lists::FilterSet::new(false);
        filter_set
            .add_filter("||example.com^", Default::default())
            .unwrap();
        let engine = crate::Engine::from_filter_set(filter_set, true);
        let block = engine.check_network_request(
            &crate::request::Request::new("https://example.com", "https://example.com", "document")
                .unwrap(),
        );
        assert_eq!(
            block.filter,
            Some("100000001100110001111111111111".to_string())
        );
    }
}

#[cfg(test)]
mod legacy_rule_parsing_tests {
    use crate::blocker::{Blocker, BlockerOptions};
    use crate::filters::network::NetworkFilterMaskHelper;
    use crate::lists::{parse_filters, FilterFormat, ParseOptions};
    use crate::test_utils::rules_from_lists;

    struct ListCounts {
        pub filters: usize,
        pub cosmetic_filters: usize,
        pub exceptions: usize,
        pub duplicates: usize,
    }

    impl std::ops::Add<ListCounts> for ListCounts {
        type Output = ListCounts;

        fn add(self, other: ListCounts) -> Self::Output {
            ListCounts {
                filters: self.filters + other.filters,
                cosmetic_filters: self.cosmetic_filters + other.cosmetic_filters,
                exceptions: self.exceptions + other.exceptions,
                duplicates: 0, // Don't bother trying to calculate - lists could have cross-duplicated entries
            }
        }
    }

    // The number of loaded rules differs from the text files due to:
    // * not handling (and not including) filters with the following options:
    //   - $popup
    //   - $elemhide
    // * not handling document/subdocument options;
    // * the optimizer that merges multiple rules into one;
    const EASY_LIST: ListCounts = ListCounts {
        filters: 47781 - 674,
        cosmetic_filters: if cfg!(feature = "css-validation") {
            23784
        } else {
            23801
        },
        exceptions: 674,
        duplicates: 0,
    };
    // differences in counts explained by hashset size underreporting as detailed in the next two cases
    const EASY_PRIVACY: ListCounts = ListCounts {
        filters: 54357 - 762, // total - exceptions
        cosmetic_filters: 29,
        exceptions: 762,
        duplicates: 2,
    };
    // ublockUnbreak = { 4, 8, 0, 94 };
    // differences in counts explained by client.hostAnchoredExceptionHashSet->GetSize() underreporting when compared to client.numHostAnchoredExceptionFilters
    const UBLOCK_UNBREAK: ListCounts = ListCounts {
        filters: 4,
        cosmetic_filters: 8,
        exceptions: 98,
        duplicates: 0,
    };
    // braveUnbreak = { 31, 0, 0, 4 };
    // differences in counts explained by client.hostAnchoredHashSet->GetSize() underreporting when compared to client.numHostAnchoredFilters
    const BRAVE_UNBREAK: ListCounts = ListCounts {
        filters: 32,
        cosmetic_filters: 0,
        exceptions: 4,
        duplicates: 0,
    };
    // disconnectSimpleMalware = { 2450, 0, 0, 0 };
    const DISCONNECT_SIMPLE_MALWARE: ListCounts = ListCounts {
        filters: 2450,
        cosmetic_filters: 0,
        exceptions: 0,
        duplicates: 0,
    };
    // spam404MainBlacklist = { 5629, 166, 0, 0 };
    const SPAM_404_MAIN_BLACKLIST: ListCounts = ListCounts {
        filters: 5629,
        cosmetic_filters: 166,
        exceptions: 0,
        duplicates: 0,
    };
    const MALWARE_DOMAIN_LIST: ListCounts = ListCounts {
        filters: 1104,
        cosmetic_filters: 0,
        exceptions: 0,
        duplicates: 3,
    };
    const MALWARE_DOMAINS: ListCounts = ListCounts {
        filters: 26853,
        cosmetic_filters: 0,
        exceptions: 0,
        duplicates: 48,
    };

    fn check_list_counts(
        rule_lists: impl IntoIterator<Item = impl AsRef<str>>,
        format: FilterFormat,
        expectation: ListCounts,
    ) {
        let rules = rules_from_lists(rule_lists);

        let (network_filters, cosmetic_filters) = parse_filters(
            rules,
            true,
            ParseOptions {
                format,
                ..Default::default()
            },
        );

        assert_eq!(
            (
                network_filters.len(),
                network_filters.iter().filter(|f| f.is_exception()).count(),
                cosmetic_filters.len()
            ),
            (
                expectation.filters + expectation.exceptions,
                expectation.exceptions,
                expectation.cosmetic_filters
            ),
            "Number of collected filters does not match expectation"
        );

        let blocker_options = BlockerOptions {
            enable_optimizations: false, // optimizations will reduce number of rules
        };

        let blocker = Blocker::new(network_filters, &blocker_options);

        // Some filters in the filter_map are pointed at by multiple tokens, increasing the total number of items
        assert!(
            blocker.exceptions().get_filter_map().total_size()
                + blocker.generic_hide().get_filter_map().total_size()
                >= expectation.exceptions,
            "Number of collected exceptions does not match expectation"
        );

        assert!(
            blocker.filters().get_filter_map().total_size()
                + blocker.importants().get_filter_map().total_size()
                + blocker.redirects().get_filter_map().total_size()
                + blocker.csp().get_filter_map().total_size()
                >= expectation.filters - expectation.duplicates,
            "Number of collected network filters does not match expectation"
        );
    }

    #[test]
    fn parse_easylist() {
        check_list_counts(
            ["./data/easylist.to/easylist/easylist.txt"],
            FilterFormat::Standard,
            EASY_LIST,
        );
    }

    #[test]
    fn parse_easyprivacy() {
        check_list_counts(
            ["./data/easylist.to/easylist/easyprivacy.txt"],
            FilterFormat::Standard,
            EASY_PRIVACY,
        );
    }

    #[test]
    fn parse_ublock_unbreak() {
        check_list_counts(
            ["./data/test/ublock-unbreak.txt"],
            FilterFormat::Standard,
            UBLOCK_UNBREAK,
        );
    }

    #[test]
    fn parse_brave_unbreak() {
        check_list_counts(
            ["./data/test/brave-unbreak.txt"],
            FilterFormat::Standard,
            BRAVE_UNBREAK,
        );
    }

    #[test]
    fn parse_brave_disconnect_simple_malware() {
        check_list_counts(
            ["./data/test/disconnect-simple-malware.txt"],
            FilterFormat::Standard,
            DISCONNECT_SIMPLE_MALWARE,
        );
    }

    #[test]
    fn parse_spam404_main_blacklist() {
        check_list_counts(
            ["./data/test/spam404-main-blacklist.txt"],
            FilterFormat::Standard,
            SPAM_404_MAIN_BLACKLIST,
        );
    }

    #[test]
    fn parse_malware_domain_list() {
        check_list_counts(
            ["./data/test/malwaredomainlist.txt"],
            FilterFormat::Hosts,
            MALWARE_DOMAIN_LIST,
        );
    }

    #[test]
    fn parse_malware_domain_list_just_hosts() {
        check_list_counts(
            ["./data/test/malwaredomainlist_justhosts.txt"],
            FilterFormat::Hosts,
            MALWARE_DOMAIN_LIST,
        );
    }

    #[test]
    fn parse_malware_domains() {
        check_list_counts(
            ["./data/test/malwaredomains.txt"],
            FilterFormat::Hosts,
            MALWARE_DOMAINS,
        );
    }

    #[test]
    fn parse_multilist() {
        let expectation = EASY_LIST + EASY_PRIVACY + UBLOCK_UNBREAK + BRAVE_UNBREAK;
        check_list_counts(
            [
                "./data/easylist.to/easylist/easylist.txt",
                "./data/easylist.to/easylist/easyprivacy.txt",
                "./data/test/ublock-unbreak.txt",
                "./data/test/brave-unbreak.txt",
            ],
            FilterFormat::Standard,
            expectation,
        )
    }

    #[test]
    fn parse_malware_multilist() {
        let expectation = SPAM_404_MAIN_BLACKLIST + DISCONNECT_SIMPLE_MALWARE;
        check_list_counts(
            [
                "./data/test/spam404-main-blacklist.txt",
                "./data/test/disconnect-simple-malware.txt",
            ],
            FilterFormat::Standard,
            expectation,
        )
    }

    #[test]
    fn parse_hosts_formats() {
        let mut expectation = MALWARE_DOMAIN_LIST + MALWARE_DOMAINS;
        expectation.duplicates = 69;
        check_list_counts(
            [
                "./data/test/malwaredomainlist.txt",
                "./data/test/malwaredomains.txt",
            ],
            FilterFormat::Hosts,
            expectation,
        )
    }
}
