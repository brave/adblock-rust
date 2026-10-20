#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::resources::MimeType;
    use crate::{
        FilterSet, lists::FilterFormat, lists::ParseOptions, test_utils::rules_from_lists,
    };
    use base64::{engine::Engine as _, prelude::BASE64_STANDARD};
    use seahash::hash;

    fn hash_without_header(data: &[u8]) -> u64 {
        let data_without_header = &data[crate::data_format::HEADER_PREFIX_LENGTH..];
        hash(data_without_header)
    }

    #[test]
    #[allow(deprecated)]
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

        let mut engine = Engine::new_with_list_text(filters.join("\n"));
        engine.enable_tags(&["stuff"]);
        engine.enable_tags(&["brian"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let request = Request::new(url, "", "", "").unwrap();
            let matched_rule = engine.check_network_request(&request);
            if expected_result {
                assert!(matched_rule.should_block(), "Expected match for {url}");
            } else {
                assert!(
                    !matched_rule.should_block(),
                    "Expected no match for {}, matched with {:?}",
                    url,
                    matched_rule.filter
                );
            }
        });
    }

    #[test]
    #[allow(deprecated)]
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

        let mut engine = Engine::new_with_list_text(filters.join("\n"));
        engine.enable_tags(&["brian", "stuff"]);
        engine.disable_tags(&["stuff"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let request = Request::new(url, "", "", "").unwrap();
            let matched_rule = engine.check_network_request(&request);
            if expected_result {
                assert!(matched_rule.should_block(), "Expected match for {url}");
            } else {
                assert!(
                    !matched_rule.should_block(),
                    "Expected no match for {}, matched with {:?}",
                    url,
                    matched_rule.filter
                );
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn exception_tags_inactive_by_default() {
        let filters = [
            "adv",
            "||brianbondy.com/$tag=brian",
            "@@||brianbondy.com/$tag=brian",
        ];
        let url_results = [
            ("http://example.com/advert.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brianbondy.com/advert", true),
        ];

        let engine = Engine::new_with_list_text(filters.join("\n"));

        url_results.into_iter().for_each(|(url, expected_result)| {
            let request = Request::new(url, "", "", "").unwrap();
            let matched_rule = engine.check_network_request(&request);
            if expected_result {
                assert!(matched_rule.should_block(), "Expected match for {url}");
            } else {
                assert!(
                    !matched_rule.should_block(),
                    "Expected no match for {}, matched with {:?}",
                    url,
                    matched_rule.filter
                );
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn exception_tags_works() {
        let filters = [
            "adv",
            "||brianbondy.com/$tag=brian",
            "@@||brianbondy.com/$tag=brian",
        ];
        let url_results = [
            ("http://example.com/advert.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brianbondy.com/advert", false),
        ];

        let mut engine = Engine::new_with_list_text(filters.join("\n"));
        engine.enable_tags(&["brian", "stuff"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let request = Request::new(url, "", "", "").unwrap();
            let matched_rule = engine.check_network_request(&request);
            if expected_result {
                assert!(matched_rule.should_block(), "Expected match for {url}");
            } else {
                assert!(
                    !matched_rule.should_block(),
                    "Expected no match for {}, matched with {:?}",
                    url,
                    matched_rule.filter
                );
            }
        });
    }

    #[test]
    #[allow(deprecated)]
    fn serialization_retains_tags() {
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

        let mut engine = Engine::new_with_list_text(filters.join("\n"));
        engine.enable_tags(&["stuff"]);
        engine.enable_tags(&["brian"]);
        let serialized = engine.serialize();
        let mut deserialized_engine = Engine::default();
        deserialized_engine.enable_tags(&["stuff"]);
        deserialized_engine.deserialize(&serialized).unwrap();

        url_results.into_iter().for_each(|(url, expected_result)| {
            let request = Request::new(url, "", "", "").unwrap();
            let matched_rule = deserialized_engine.check_network_request(&request);
            if expected_result {
                assert!(matched_rule.should_block(), "Expected match for {url}");
            } else {
                assert!(
                    !matched_rule.should_block(),
                    "Expected no match for {}, matched with {:?}",
                    url,
                    matched_rule.filter
                );
            }
        });
    }

    const HASH_MISMATCH_MSG: &str = r#"
      A change has been detected in the serialized format! If the change is intentional:
      1. Update ADBLOCK_RUST_DAT_VERSION before updating the expected hashes
      2. DON'T rely on backwards compatibility with the old format
      Backwards compatibility isn't covered by the tests"#;

    #[test]
    fn deserialization_generate_simple() {
        let mut engine = Engine::new_with_list_text("ad-banner");
        let data = engine.serialize().to_vec();
        const EXPECTED_HASH: u64 = 16174589584358908453;
        assert_eq!(
            hash_without_header(&data),
            EXPECTED_HASH,
            "{HASH_MISMATCH_MSG}"
        );
        engine.deserialize(&data).unwrap();
    }

    #[test]
    #[allow(deprecated)]
    fn deserialization_generate_tags() {
        let mut engine = Engine::new_with_list_text("ad-banner$tag=abc");
        engine.use_tags(&["abc"]);
        let data = engine.serialize().to_vec();
        const EXPECTED_HASH: u64 = 17947791698945847098;
        assert_eq!(
            hash_without_header(&data),
            EXPECTED_HASH,
            "{HASH_MISMATCH_MSG}"
        );
        engine.deserialize(&data).unwrap();
    }

    #[test]
    fn deserialization_generate_resources() {
        let mut engine = Engine::new_with_list_text("ad-banner$redirect=nooptext");

        engine.use_resources([
            Resource::simple("nooptext", MimeType::TextPlain, ""),
            Resource::simple("noopcss", MimeType::TextCss, ""),
        ]);

        let serialized = engine.serialize().to_vec();
        println!("Engine serialized: {serialized:?}");
        engine.deserialize(&serialized).unwrap();
    }

    #[test]
    fn deserialization_brave_list() {
        let rules = rules_from_lists(["data/brave/brave-main-list.txt"]);
        let mut engine = Engine::new_with_list_text(rules);
        let data = engine.serialize();

        #[cfg(feature = "debug-info")]
        {
            let debug_info = engine.get_debug_info();
            let low_bound = 8_200_000;
            let high_bound = 8_900_000;
            assert!(
                debug_info.flatbuffer_size >= low_bound,
                "Expected size >= {} bytes, got {}",
                low_bound,
                debug_info.flatbuffer_size
            );
            assert!(
                debug_info.flatbuffer_size <= high_bound,
                "Expected size <= {} bytes, got {}",
                high_bound,
                debug_info.flatbuffer_size
            );

            assert_eq!(debug_info.source_info.len(), 1);
            assert_eq!(
                debug_info.source_info[0].title,
                Some("uBlock filters".to_string())
            );
            assert_eq!(
                debug_info.source_info[0].homepage,
                Some("https://github.com/uBlockOrigin/uAssets".to_string())
            );
            assert_eq!(debug_info.source_info[0].network_filter_count, 124261);
            assert_eq!(debug_info.source_info[0].cosmetic_filter_count, 42775);
        }
        let expected_hash: u64 = if cfg!(feature = "css-validation") {
            17902877162789739649
        } else {
            4788447722899950551
        };

        assert_eq!(
            hash_without_header(&data),
            expected_hash,
            "{HASH_MISMATCH_MSG}"
        );

        use crate::flatbuffers::unsafe_tools::root_as_engine_calls;

        let initial_count = root_as_engine_calls::count();
        engine.deserialize(&data).unwrap();
        assert_eq!(
            root_as_engine_calls::count(),
            initial_count,
            "matching crate version should skip fb::root_as_engine"
        );

        let mut verified = data.clone();
        verified[crate::data_format::HEADER_PREFIX_LENGTH - 1] ^= 1;
        engine.deserialize(&verified).unwrap();
        assert_eq!(
            root_as_engine_calls::count(),
            initial_count + 1,
            "mismatched crate version should call fb::root_as_engine"
        );
    }

    #[test]
    fn deserialize_rejects_invalid_flatbuffer() {
        let mut garbage = crate::data_format::serialize_dat_file(b"not a flatbuffer");
        garbage[crate::data_format::HEADER_PREFIX_LENGTH - 1] ^= 1;
        assert!(matches!(
            Engine::default().deserialize(&garbage),
            Err(DeserializationError::FlatBufferParsingError(_))
        ));
    }

    #[test]
    fn redirect_resource_insertion_works() {
        let mut engine = Engine::new_with_list_text(
            ["ad-banner$redirect=nooptext", "script.js$redirect=noop.js"].join("\n"),
        );

        let script = r#"
(function() {
	;
})();

        "#;
        let mut resources = [
            Resource::simple("nooptext", MimeType::TextPlain, ""),
            Resource::simple("noopjs", MimeType::ApplicationJavascript, script),
        ];
        resources[1].aliases.push("noop.js".to_string());
        engine.use_resources(resources);

        let url = "http://example.com/ad-banner.gif";
        let request = Request::new(url, "", "", "").unwrap();
        let matched_rule = engine.check_network_request(&request);
        assert!(matched_rule.should_block(), "Expected match for {url}");
        assert_eq!(
            matched_rule.redirect,
            Some("data:text/plain;base64,".to_owned()),
            "Expected redirect to contain resource"
        );

        let url = "http://example.com/script.js";
        let request = Request::new(url, "", "", "").unwrap();
        let matched_rule = engine.check_network_request(&request);
        assert!(matched_rule.should_block(), "Expected match for {url}");
        assert_eq!(
            matched_rule.redirect,
            Some(format!(
                "data:application/javascript;base64,{}",
                BASE64_STANDARD.encode(script)
            )),
            "Expected redirect to contain resource"
        );
    }

    #[test]
    fn document() {
        let filters = ["||example.com$document", "@@||sub.example.com$document"];

        let engine = Engine::new_with_list_text(filters.join("\n"));

        assert!(
            engine
                .check_network_request(
                    &Request::new("https://example.com", "https://example.com", "document", "")
                        .unwrap()
                )
                .should_block()
        );
        assert!(
            !engine
                .check_network_request(
                    &Request::new("https://example.com", "https://example.com", "script", "")
                        .unwrap()
                )
                .should_block()
        );
        assert!(
            engine
                .check_network_request(
                    &Request::new(
                        "https://sub.example.com",
                        "https://sub.example.com",
                        "document",
                        ""
                    )
                    .unwrap()
                )
                .exception
                .is_some()
        );
    }

    #[test]
    fn implicit_all() {
        {
            let engine = Engine::new_with_list_text("||example.com^");
            assert!(
                engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text("||example.com^$first-party");
            assert!(
                engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text("||example.com^$script");
            assert!(
                !engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text("||example.com^$~script");
            assert!(
                !engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text(
                ["||example.com^$document", "@@||example.com^$generichide"].join("\n"),
            );
            assert!(
                engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let mut filter_set = FilterSet::new(false);
            filter_set.add_filter_list(
                "example.com".to_string(),
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            let engine = Engine::new_with_filter_set(filter_set);
            assert!(
                engine
                    .check_network_request(
                        &Request::new("https://example.com", "https://example.com", "document", "")
                            .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text("||example.com/path");
            assert!(
                !engine
                    .check_network_request(
                        &Request::new(
                            "https://example.com/path",
                            "https://example.com/path",
                            "document",
                            ""
                        )
                        .unwrap()
                    )
                    .should_block()
            );
        }
        {
            let engine = Engine::new_with_list_text("||example.com/path^");
            assert!(
                !engine
                    .check_network_request(
                        &Request::new(
                            "https://example.com/path",
                            "https://example.com/path",
                            "document",
                            ""
                        )
                        .unwrap()
                    )
                    .should_block()
            );
        }
    }

    #[test]
    fn explicit_all() {
        let engine = Engine::new_with_list_text("*$all,domain=rarvinzp.click|ytrqcxat.click");
        for content_type in [
            "script",
            "document",
            "subdocument",
            "font",
            "xmlhttprequest",
        ] {
            assert!(
                engine
                    .check_network_request(
                        &Request::new(
                            "https://example.com",
                            "https://rarvinzp.click",
                            content_type,
                            ""
                        )
                        .unwrap()
                    )
                    .should_block()
            );
        }
    }

    #[test]
    fn generichide() {
        let filters = [
            "##.donotblock",
            "##a[href=\"generic.com\"]",
            "@@||example.com$generichide",
            "example.com##.block",
            "@@||example2.com/test.html$generichide",
            "example2.com##.block",
        ]
        .join("\n");
        let url_results = [
            ("https://example.com", vec![".block"], true),
            ("https://example.com/test.html", vec![".block"], true),
            (
                "https://example2.com",
                vec![".block", "a[href=\"generic.com\"]"],
                false,
            ),
            ("https://example2.com/test.html", vec![".block"], true),
        ];

        let engine = Engine::new_with_list_text(filters);

        url_results
            .into_iter()
            .for_each(|(url, expected_result, expected_generichide)| {
                let result = engine.url_cosmetic_resources(url);
                assert_eq!(
                    result.hide_selectors,
                    expected_result
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<HashSet<_>>()
                );
                assert_eq!(result.generichide, expected_generichide);
            });
    }

    #[test]
    fn important_redirect() {
        let mut filter_set = FilterSet::new(true);
        filter_set.add_filters([
            "||addthis.com^$important,3p,domain=~missingkids.com|~missingkids.org|~sainsburys.jobs|~sitecore.com|~amd.com",
            "||addthis.com/*/addthis_widget.js$script,redirect=addthis.com/addthis_widget.js",
        ], Default::default());
        let mut engine = Engine::new_with_filter_set(filter_set);

        engine.use_resources([Resource::simple(
            "addthis.com/addthis_widget.js",
            MimeType::ApplicationJavascript,
            "window.addthis = undefined",
        )]);

        let request = Request::new("https://s7.addthis.com/js/250/addthis_widget.js?pub=resto", "https://www.rhmodern.com/catalog/product/product.jsp?productId=prod14970086&categoryId=cat7150028", "script", "").unwrap();
        let result = engine.check_network_request(&request);

        assert!(result.redirect.is_some());
    }

    #[test]
    fn check_match_case_regex_filtering() {
        {
            // match case without regex is discarded
            let engine = Engine::new_with_list_text("ad.png$match-case");
            let request = Request::new(
                "https://example.com/ad.png",
                "https://example.com",
                "image",
                "",
            )
            .unwrap();
            assert!(!engine.check_network_request(&request).should_block());
        }
        {
            // /^https:\/\/[0-9a-z]{3,}\.[-a-z]{10,}\.(?:li[fv]e|top|xyz)\/[a-z]{8}\/\?utm_campaign=\w{40,}/$doc,match-case,domain=life|live|top|xyz
            let engine = Engine::new_with_list_text(
                r#"/^https:\/\/[0-9a-z]{3,}\.[-a-z]{10,}\.(?:li[fv]e|top|xyz)\/[a-z]{8}\/\?utm_campaign=\w{40,}/$doc,match-case,domain=life|live|top|xyz"#,
            );
            let request = Request::new("https://www.exampleaaa.xyz/testtest/?utm_campaign=aaaaaaaaaabbbbbbbbbbccccccccccdddddddddd", "https://www.exampleaaa.xyz/testtest/?utm_campaign=aaaaaaaaaabbbbbbbbbbccccccccccdddddddddd", "document", "").unwrap();
            assert!(engine.check_network_request(&request).should_block());
        }
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https?:\/\/((?!www)[a-z]{3,}|\d{2})?\.?[-0-9a-z]{6,}\.[a-z]{2,6}\/(?:[a-z]{6,8}\/)?\/?\?u=[0-9a-z]{7}&o=[0-9a-z]{7}/$doc,frame,match-case,domain=buzz|com|de|fun|guru|info|life|live|mobi|online|pw|site|space|top|us|xyz
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/((?!www)[a-z]{3,}|\d{2})?\.?[-0-9a-z]{6,}\.[a-z]{2,6}\/(?:[a-z]{6,8}\/)?\/?\?u=[0-9a-z]{7}&o=[0-9a-z]{7}/$doc,frame,match-case,domain=buzz|com|de|fun|guru|info|life|live|mobi|online|pw|site|space|top|us|xyz"#], Default::default());
            let request = Request::new("https://example.com/aaaaaa/?u=aaaaaaa&o=bbbbbbb", "https://example.com/aaaaaa/?u=aaaaaaa&o=bbbbbbb", "document", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https:\/\/(?:www\d\.)?[-a-z]{6,}\.(?:com|info|net|org)\/(?=[-_a-zA-Z]{0,42}\d)(?=[-_0-9a-z]{0,42}[A-Z])[-_0-9a-zA-Z]{43}\/\?cid=[-_0-9a-zA-Z]{16,36}(?:&qs\d=\S+)?&sid=[_0-9a-f]{1,32}$/$doc,match-case,domain=com|info|net|org
            let engine = Engine::from_rules_debug([r#"/^https:\/\/(?:www\d\.)?[-a-z]{6,}\.(?:com|info|net|org)\/(?=[-_a-zA-Z]{0,42}\d)(?=[-_0-9a-z]{0,42}[A-Z])[-_0-9a-zA-Z]{43}\/\?cid=[-_0-9a-zA-Z]{16,36}(?:&qs\d=\S+)?&sid=[_0-9a-f]{1,32}$/$doc,match-case,domain=com|info|net|org"#], Default::default());
            let request = Request::new("https://www3.example.com/aaaaaaaaaabbbbbbbbbbccccccccccddddddddddAA5/?cid=aaaaaaaaaabbbbbb&qs5=\n&sid=a", "https://www3.example.com/aaaaaaaaaabbbbbbbbbbccccccccccddddddddddAA5/?cid=aaaaaaaaaabbbbbb&qs5=\n&sid=a", "document", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https:\/\/(?:www\d\.)?[-a-z]{6,}\.(?:com|info|net|org)\/(?=[-_a-zA-Z]{0,42}\d)(?=[-_0-9a-z]{0,42}[A-Z])[-_0-9a-zA-Z]{43}\/\?sid=[_0-9a-f]{1,32}(?:&qs\d=\S+)?&cid=[-_0-9a-zA-Z]{16,36}$/$doc,match-case,domain=com|info|net|org
            let engine = Engine::from_rules_debug([r#"/^https:\/\/(?:www\d\.)?[-a-z]{6,}\.(?:com|info|net|org)\/(?=[-_a-zA-Z]{0,42}\d)(?=[-_0-9a-z]{0,42}[A-Z])[-_0-9a-zA-Z]{43}\/\?cid=[-_0-9a-zA-Z]{16,36}(?:&qs\d=\S+)?&sid=[_0-9a-f]{1,32}$/$doc,match-case,domain=com|info|net|org"#], Default::default());
            let request = Request::new("https://www3.example.com/aaaaaaaaaabbbbbbbbbbccccccccccddddddddddAA5/?sid=1&qs1=\n&cid=aaaaaaaaaabbbbbb", "https://www3.example.com/aaaaaaaaaabbbbbbbbbbccccccccccddddddddddAA5/?sid=1&qs1=\n&cid=aaaaaaaaaabbbbbb", "document", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        {
            // /^http:\/\/[a-z]{5}\.[a-z]{5}\.com\/[a-z]{10}\.apk$/$doc,match-case,domain=com
            let engine = Engine::new_with_list_text(
                r#"/^http:\/\/[a-z]{5}\.[a-z]{5}\.com\/[a-z]{10}\.apk$/$doc,match-case,domain=com"#,
            );
            let request = Request::new(
                "http://abcde.abcde.com/aaaaabbbbb.apk",
                "http://abcde.abcde.com/aaaaabbbbb.apk",
                "document",
                "",
            )
            .unwrap();
            assert!(engine.check_network_request(&request).should_block());
        }
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /\/[A-Z]\/[-0-9a-z]{5,}\.com\/(?:[0-9a-f]{2}\/){3}[0-9a-f]{32}\.js$/$script,1p,match-case
            let engine = Engine::from_rules_debug([r#"/\/[A-Z]\/[-0-9a-z]{5,}\.com\/(?:[0-9a-f]{2}\/){3}[0-9a-f]{32}\.js$/$script,1p,match-case"#], Default::default());
            let request = Request::new("/A/aaaaa.com/aa/bb/cc/aaaaaaaabbbbbbbbccccccccdddddddd.js", "/A/aaaaa.com/aa/bb/cc/aaaaaaaabbbbbbbbccccccccdddddddd.js", "script", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https?:\/\/(?:[a-z]{2}\.)?[0-9a-z]{7,16}\.com\/[a-z](?=[a-z]{0,25}[0-9A-Z])[0-9a-zA-Z]{3,26}\/(?:[1-5]\d{4}|[3-9]\d{3})\??(?:_=\d+|v=\d)?$/$frame,script,xhr,popup,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/(?:[a-z]{2}\.)?[0-9a-z]{7,16}\.com\/[a-z](?=[a-z]{0,25}[0-9A-Z])[0-9a-zA-Z]{3,26}\/(?:[1-5]\d{4}|[3-9]\d{3})\??(?:_=\d+|v=\d)?$/$frame,script,xhr,popup,3p,match-case"#], Default::default());
            let request = Request::new("https://aa.example.com/aAaaa/12222", "https://aa.example.net/aAaaa/12222", "frame", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https?:\/\/(?:[a-z]{2}\.)?[0-9a-z]{7,16}\.website\/[a-z](?=[a-z]{0,25}[0-9A-Z])[0-9a-zA-Z]{3,26}\/(?:[1-5]\d{4}|[3-9]\d{3})\??(?:_=\d+|v=\d)?$/$frame,script,xhr,popup,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/(?:[a-z]{2}\.)?[0-9a-z]{7,16}\.website\/[a-z](?=[a-z]{0,25}[0-9A-Z])[0-9a-zA-Z]{3,26}\/(?:[1-5]\d{4}|[3-9]\d{3})\??(?:_=\d+|v=\d)?$/$frame,script,xhr,popup,3p,match-case"#], Default::default());
            let request = Request::new("https://aa.example.website/aAaaa/12222", "https://aa.example.website/aAaaa/12222", "frame", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https?:\/\/[a-z]{8,15}\.top(\/(?:\d{1,5}|0NaN|articles?|browse|index|movie|news|pages?|static|view|web|wiki)){1,4}(?:\.html|\/)$/$frame,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/[a-z]{8,15}\.top(\/(?:\d{1,5}|0NaN|articles?|browse|index|movie|news|pages?|static|view|web|wiki)){1,4}(?:\.html|\/)$/$frame,3p,match-case"#], Default::default());
            let request = Request::new("https://examples.top/articles.html", "https://examples.top/articles.html", "frame", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        {
            // /^https?:\/\/[a-z]{8,15}\.top\/[a-z]{4,}\.json$/$xhr,3p,match-case
            let engine = Engine::new_with_list_text(
                r#"/^https?:\/\/[a-z]{8,15}\.top\/[a-z]{4,}\.json$/$xhr,3p,match-case"#,
            );
            let request = Request::new(
                "https://examples.top/abcd.json",
                "https://examples.com/abcd.json",
                "xhr",
                "",
            )
            .unwrap();
            assert!(engine.check_network_request(&request).should_block());
        }
        // fails - inferring unescaped `$` inside regex pattern
        /*{
            // /^https?:\/\/[a-z]{8,15}\.top\/[-a-z]{4,}\.css\?aHR0c[\/0-9a-zA-Z]{33,}=?=?$/$css,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/[a-z]{8,15}\.top\/[-a-z]{4,}\.css\?aHR0c[\/0-9a-zA-Z]{33,}=?=?$/$css,3p,match-case"#], Default::default());
            let request = Request::new("https://examples.top/abcd.css?aHR0c/aaaaaaaaaaAAAAAAAAAA000000000012==", "https://examples.com/abcd.css?aHR0c/aaaaaaaaaaAAAAAAAAAA000000000012==", "stylesheet", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - inferring unescaped `$` inside regex pattern
        /*{
            // /^https?:\/\/[a-z]{8,15}\.top\/[a-z]{4,}\.png\?aHR0c[\/0-9a-zA-Z]{33,}=?=?$/$image,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/[a-z]{8,15}\.top\/[a-z]{4,}\.png\?aHR0c[\/0-9a-zA-Z]{33,}=?=?$/$image,3p,match-case"#], Default::default());
            let request = Request::new("https://examples.top/abcd.png?aHR0c/aaaaaaaaaaAAAAAAAAAA000000000012==", "https://examples.com/abcd.png?aHR0c/aaaaaaaaaaAAAAAAAAAA000000000012==", "image", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https?:\/\/[a-z]{8,15}\.xyz(\/(?:\d{1,5}|0NaN|articles?|browse|index|movie|news|pages?|static|view|web|wiki)){1,4}(?:\.html|\/)$/$frame,3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/[a-z]{8,15}\.xyz(\/(?:\d{1,5}|0NaN|articles?|browse|index|movie|news|pages?|static|view|web|wiki)){1,4}(?:\.html|\/)$/$frame,3p,match-case"#], Default::default());
            let request = Request::new("https://examples.xyz/articles.html", "https://examples.xyz/articles.html", "frame", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        {
            // /^https?:\/\/cdn\.[a-z]{4,6}\.xyz\/app\.js$/$script,3p,match-case
            let engine = Engine::new_with_list_text(
                r#"/^https?:\/\/cdn\.[a-z]{4,6}\.xyz\/app\.js$/$script,3p,match-case"#,
            );
            let request = Request::new(
                "https://cdn.abcde.xyz/app.js",
                "https://cdn.abcde.com/app.js",
                "script",
                "",
            )
            .unwrap();
            assert!(engine.check_network_request(&request).should_block());
        }
        // fails - because of non-supported look around operator in rust regex https://github.com/rust-lang/regex/issues/127#issuecomment-154713666
        /*{
            // /^https:\/\/a\.[-0-9a-z]{4,16}\.(?:club|com?|cyou|info|net|ru|site|top?|xxx|xyz)\/(?=[a-z]{0,6}[0-9A-Z])[0-9a-zA-Z]{7}\.js$/$script,match-case
            let engine = Engine::from_rules_debug([r#"/^https:\/\/a\.[-0-9a-z]{4,16}\.(?:club|com?|cyou|info|net|ru|site|top?|xxx|xyz)\/(?=[a-z]{0,6}[0-9A-Z])[0-9a-zA-Z]{7}\.js$/$script,match-case"#], Default::default());
            let request = Request::new("https://a.abcd.club/aaaaaaA.js", "https://a.abcd.club/aaaaaaA.js", "script", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        {
            // /^https:\/\/cdn\.jsdelivr\.net\/npm\/[-a-z_]{4,22}@latest\/dist\/script\.min\.js$/$script,3p,match-case
            let engine = Engine::new_with_list_text(
                r#"/^https:\/\/cdn\.jsdelivr\.net\/npm\/[-a-z_]{4,22}@latest\/dist\/script\.min\.js$/$script,3p,match-case"#,
            );
            let request = Request::new(
                "https://cdn.jsdelivr.net/npm/abcd@latest/dist/script.min.js",
                "https://cdn.jsdelivr.com/npm/abcd@latest/dist/script.min.js",
                "script",
                "",
            )
            .unwrap();
            assert!(engine.check_network_request(&request).should_block());
        }
        // fails - inferring unescaped `$` inside regex pattern
        /*{
            // /^https?:\/\/[-.0-9a-z]+\/script\.js$/$script,1p,strict3p,match-case
            let engine = Engine::from_rules_debug([r#"/^https?:\/\/[-.0-9a-z]+\/script\.js$/$script,1p,strict3p,match-case"#], Default::default());
            let request = Request::new("https://www.example.com/script.js", "https://www.abc.com/script.js", "script", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - unicode not supported in network filter
        /*{
            let engine = Engine::from_rules_debug([r#"/tesT߶/$domain=example.com"#], Default::default());
            let request = Request::new("https://example.com/tesT߶", "https://example.com", "script", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
        // fails - unicode not supported in network filter
        /*{
            let engine = Engine::from_rules_debug([r#"/tesT߶/$domain=example.com"#], Default::default());
            let request = Request::new("https://example-tesT߶.com/tesT", "https://example.com", "script", "").unwrap();
            assert!(engine.check_network_request(&request).matched);
        }*/
    }

    #[test]
    fn scriptlet_permissions() {
        use crate::resources::{PermissionMask, ResourceType};
        const UBO_PERM: PermissionMask = PermissionMask::from_bits(0b00000001);
        const BRAVE_PERM: PermissionMask = PermissionMask::from_bits(0b00000011);

        let resources = [
            Resource::simple(
                "refresh-defuser.js",
                MimeType::ApplicationJavascript,
                "refresh-defuser",
            ),
            Resource {
                name: "trusted-set-cookie.js".to_string(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: BASE64_STANDARD.encode("trusted-set-cookie"),
                dependencies: vec![],
                permission: UBO_PERM,
            },
            Resource {
                name: "brave-fix.js".to_string(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: BASE64_STANDARD.encode("brave-fix"),
                dependencies: vec![],
                permission: BRAVE_PERM,
            },
        ];

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(
            [
                "sub1.example.com##+js(refresh-defuser)",
                "sub2.example.com##+js(trusted-set-cookie)",
                "sub3.example.com##+js(brave-fix)",
            ],
            Default::default(),
        );
        filter_set.add_filters(
            [
                "sub4.example.com##+js(refresh-defuser)",
                "sub5.example.com##+js(trusted-set-cookie)",
                "sub6.example.com##+js(brave-fix)",
            ],
            ParseOptions {
                permissions: UBO_PERM,
                ..Default::default()
            },
        );
        filter_set.add_filters(
            [
                "sub7.example.com##+js(refresh-defuser)",
                "sub8.example.com##+js(trusted-set-cookie)",
                "sub9.example.com##+js(brave-fix)",
            ],
            ParseOptions {
                permissions: BRAVE_PERM,
                ..Default::default()
            },
        );

        let mut engine = Engine::new_with_filter_set(filter_set);
        engine.use_resources(resources);

        fn wrap_try(scriptlet_content: &str) -> String {
            format!("try {{\n{scriptlet_content}\n}} catch ( e ) {{ }}\n")
        }

        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub1.example.com")
                .injected_script,
            wrap_try("refresh-defuser")
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub2.example.com")
                .injected_script,
            ""
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub3.example.com")
                .injected_script,
            ""
        );

        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub4.example.com")
                .injected_script,
            wrap_try("refresh-defuser")
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub5.example.com")
                .injected_script,
            wrap_try("trusted-set-cookie")
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub6.example.com")
                .injected_script,
            ""
        );

        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub7.example.com")
                .injected_script,
            wrap_try("refresh-defuser")
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub8.example.com")
                .injected_script,
            wrap_try("trusted-set-cookie")
        );
        assert_eq!(
            engine
                .url_cosmetic_resources("https://sub9.example.com")
                .injected_script,
            wrap_try("brave-fix")
        );
    }

    #[test]
    fn quoted_scriptlet_args() {
        use crate::resources::{MimeType, ResourceType};

        let resources = [
            Resource {
                name: "trusted-set-local-storage-item.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: BASE64_STANDARD.encode("function trustedSetLocalStorageItem(key = '', value = '') { setLocalStorageItemFn('local', true, key, value); }"),
                dependencies: vec![],
                permission: Default::default(),
            },
        ];

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters([
            r#"dailymail.co.uk##+js(trusted-set-local-storage-item, mol.ads.cmp.tcf.cache, '{"getTCData":{"cmpId":27,"cmpVersion":3,"gdprApplies":true,"tcfPolicyVersion":2,"tcString":"CPyz5QAPyz5QAAbADCENC6CgAAAAAAAAAAwIAAASjAJINW4gCLMscGaQEIoEAIgjCQggUAAFAILRAQAODgp2VgE6MIkAAAUARABAhwAQAQCAAASABCAAJAAwQAAAiAQAAAAQCAAAMCAILACgAAAABANAhRCgAECQAyIAIpTAgKgSCAFsKAAADJCQCAKgMAKARGgEACIIARGAAACwMAgBICFggABMQbBAAMACAESoBoCTEwBACDQFgBkADLAGzAPsA_ACAAEFAIwASYAp8BaAFpAOqAfIBDoCJgEiAKRAXIAyMBk4DlAI_gSKEQEwBkADLAGzAPsA_ACAAEYAJMAU8A6oB8gEOgJEAUiAuQBkYDJwHKAR_AkU.f_gAAagAAAAA","eventStatus":"useractioncomplete","cmpStatus":"loaded","isServiceSpecific":true,"useNonStandardStacks":false,"publisherCC":"GB","purposeOneTreatment":false,"addtlConsent":"1~","acmVersion":2,"molGvlVersion":"186.gb.web","nrvString":"1~","nrvVersion":1,"repromptVersion":5},"getStoredRepromptVersion":5,"hasUserConsentedToAll":false,"hasUserDissentedToAll":true,"getConsentDegree":"no","getValidTCData":{"cmpId":27,"cmpVersion":3,"gdprApplies":true,"tcfPolicyVersion":2,"tcString":"CPyz5QAPyz5QAAbADCENC6CgAAAAAAAAAAwIAAASjAJINW4gCLMscGaQEIoEAIgjCQggUAAFAILRAQAODgp2VgE6MIkAAAUARABAhwAQAQCAAASABCAAJAAwQAAAiAQAAAAQCAAAMCAILACgAAAABANAhRCgAECQAyIAIpTAgKgSCAFsKAAADJCQCAKgMAKARGgEACIIARGAAACwMAgBICFggABMQbBAAMACAESoBoCTEwBACDQFgBkADLAGzAPsA_ACAAEFAIwASYAp8BaAFpAOqAfIBDoCJgEiAKRAXIAyMBk4DlAI_gSKEQEwBkADLAGzAPsA_ACAAEYAJMAU8A6oB8gEOgJEAUiAuQBkYDJwHKAR_AkU.f_gAAagAAAAA","listenerId":1,"eventStatus":"useractioncomplete","cmpStatus":"loaded","isServiceSpecific":true,"useNonStandardStacks":false,"publisherCC":"GB","purposeOneTreatment":false,"addtlConsent":"1~","acmVersion":2,"molGvlVersion":"186.gb.web","nrvString":"1~","nrvVersion":1,"repromptVersion":5}}')"#,
            // invalid - unclosed quoted arg
            r#"example.com##+js(trusted-set-local-storage-item, "test)"#,
            // invalid - closing quote does not surround the argument
            r#"example.com##+js(trusted-set-local-storage-item, "test"test, 3)"#,
        ], Default::default());

        let mut engine = Engine::new_with_filter_set(filter_set);
        engine.use_resources(resources);

        assert_eq!(engine.url_cosmetic_resources("https://dailymail.co.uk").injected_script, r#"function trustedSetLocalStorageItem(key = '', value = '') { setLocalStorageItemFn('local', true, key, value); }
try {
trustedSetLocalStorageItem("mol.ads.cmp.tcf.cache", "{\"getTCData\":{\"cmpId\":27,\"cmpVersion\":3,\"gdprApplies\":true,\"tcfPolicyVersion\":2,\"tcString\":\"CPyz5QAPyz5QAAbADCENC6CgAAAAAAAAAAwIAAASjAJINW4gCLMscGaQEIoEAIgjCQggUAAFAILRAQAODgp2VgE6MIkAAAUARABAhwAQAQCAAASABCAAJAAwQAAAiAQAAAAQCAAAMCAILACgAAAABANAhRCgAECQAyIAIpTAgKgSCAFsKAAADJCQCAKgMAKARGgEACIIARGAAACwMAgBICFggABMQbBAAMACAESoBoCTEwBACDQFgBkADLAGzAPsA_ACAAEFAIwASYAp8BaAFpAOqAfIBDoCJgEiAKRAXIAyMBk4DlAI_gSKEQEwBkADLAGzAPsA_ACAAEYAJMAU8A6oB8gEOgJEAUiAuQBkYDJwHKAR_AkU.f_gAAagAAAAA\",\"eventStatus\":\"useractioncomplete\",\"cmpStatus\":\"loaded\",\"isServiceSpecific\":true,\"useNonStandardStacks\":false,\"publisherCC\":\"GB\",\"purposeOneTreatment\":false,\"addtlConsent\":\"1~\",\"acmVersion\":2,\"molGvlVersion\":\"186.gb.web\",\"nrvString\":\"1~\",\"nrvVersion\":1,\"repromptVersion\":5},\"getStoredRepromptVersion\":5,\"hasUserConsentedToAll\":false,\"hasUserDissentedToAll\":true,\"getConsentDegree\":\"no\",\"getValidTCData\":{\"cmpId\":27,\"cmpVersion\":3,\"gdprApplies\":true,\"tcfPolicyVersion\":2,\"tcString\":\"CPyz5QAPyz5QAAbADCENC6CgAAAAAAAAAAwIAAASjAJINW4gCLMscGaQEIoEAIgjCQggUAAFAILRAQAODgp2VgE6MIkAAAUARABAhwAQAQCAAASABCAAJAAwQAAAiAQAAAAQCAAAMCAILACgAAAABANAhRCgAECQAyIAIpTAgKgSCAFsKAAADJCQCAKgMAKARGgEACIIARGAAACwMAgBICFggABMQbBAAMACAESoBoCTEwBACDQFgBkADLAGzAPsA_ACAAEFAIwASYAp8BaAFpAOqAfIBDoCJgEiAKRAXIAyMBk4DlAI_gSKEQEwBkADLAGzAPsA_ACAAEYAJMAU8A6oB8gEOgJEAUiAuQBkYDJwHKAR_AkU.f_gAAagAAAAA\",\"listenerId\":1,\"eventStatus\":\"useractioncomplete\",\"cmpStatus\":\"loaded\",\"isServiceSpecific\":true,\"useNonStandardStacks\":false,\"publisherCC\":\"GB\",\"purposeOneTreatment\":false,\"addtlConsent\":\"1~\",\"acmVersion\":2,\"molGvlVersion\":\"186.gb.web\",\"nrvString\":\"1~\",\"nrvVersion\":1,\"repromptVersion\":5}}")
} catch ( e ) { }
"#.to_owned());

        assert_eq!(
            engine
                .url_cosmetic_resources("https://example.com")
                .injected_script,
            ""
        );
    }

    #[test]
    fn method_option_blocks_post_xhr_only() {
        let engine =
            Engine::new_with_list_text("||perplexity.ai/rest/metrics/collect^$xhr,1p,method=post");
        let url = "https://perplexity.ai/rest/metrics/collect?foo=bar";
        let source = "https://perplexity.ai/page";

        let post = Request::new(url, source, "xhr", "post").unwrap();
        assert!(
            engine.check_network_request(&post).should_block(),
            "POST xhr 1p should match"
        );

        let get = Request::new(url, source, "xhr", "get").unwrap();
        assert!(
            !engine.check_network_request(&get).should_block(),
            "GET xhr should not match method=post rule"
        );

        let post_3p = Request::new(url, "https://other.com/page", "xhr", "post").unwrap();
        assert!(
            !engine.check_network_request(&post_3p).should_block(),
            "POST xhr 3p should not match 1p rule"
        );

        let post_no_method = Request::new(url, source, "xhr", "").unwrap();
        assert!(
            !engine.check_network_request(&post_no_method).should_block(),
            "missing method should not match method=post rule"
        );
    }

    #[test]
    fn method_option_exception_head_get() {
        let engine = Engine::new_with_list_text(
            "||tracker.example.com^$xhr\n@@*$xhr,method=head|get,domain=app.axenthost.com,3p",
        );
        let url = "https://tracker.example.com/pixel";
        let source = "https://app.axenthost.com/page";

        for method in ["get", "head"] {
            let request = Request::new(url, source, "xhr", method).unwrap();
            assert!(
                !engine.check_network_request(&request).should_block(),
                "{method} xhr should be excepted"
            );
        }

        let post = Request::new(url, source, "xhr", "post").unwrap();
        assert!(
            engine.check_network_request(&post).should_block(),
            "POST xhr should still be blocked"
        );
    }

    #[test]
    fn expensive_to_option_is_ignored() {
        let engine = Engine::new_with_list_text(
            [
                "*$script,to=com",
                "*$script,to=~evil.com",
                "*$script,to=com|net",
                "*$script,to=ads.example|net",
                r"/banner\.gif/$to=ads.example",
                "/somepath/*$script,to=com",
            ]
            .join("\n"),
        );

        let broad =
            Request::new("https://other.com/a.js", "https://page.com", "script", "").unwrap();
        assert!(
            !engine.check_network_request(&broad).should_block(),
            "too-common / exclude-only $to= must not match"
        );

        let specific =
            Request::new("https://ads.example/a.js", "https://page.com", "script", "").unwrap();
        assert!(
            engine.check_network_request(&specific).should_block(),
            "specific $to= should match after stripping broad labels"
        );

        let regex_specific =
            Request::new("https://ads.example/banner.gif", "https://page.com", "", "").unwrap();
        assert!(
            engine.check_network_request(&regex_specific).should_block(),
            "regex + specific $to= should still match"
        );

        let path_com = Request::new(
            "https://cdn.example.com/somepath/x.js",
            "https://page.com",
            "script",
            "",
        )
        .unwrap();
        assert!(
            engine.check_network_request(&path_com).should_block(),
            "path token + broad $to=com should match .com destinations"
        );

        let path_org = Request::new(
            "https://cdn.example.org/somepath/x.js",
            "https://page.com",
            "script",
            "",
        )
        .unwrap();
        assert!(
            !engine.check_network_request(&path_org).should_block(),
            "path token + broad $to=com must not match non-.com destinations"
        );
    }

    #[test]
    fn exclude_only_to_option_with_from_is_enforced() {
        let engine = Engine::new_with_list_text(
            "*$script,3p,from=ovagames.com,to=~facebook.net|~fbcdn.net".to_string(),
        );

        let blocked = Request::new(
            "https://example.com/a.js",
            "https://ovagames.com",
            "script",
            "",
        )
        .unwrap();
        assert!(engine.check_network_request(&blocked).should_block());

        let excluded = Request::new(
            "https://cdn.facebook.net/a.js",
            "https://ovagames.com",
            "script",
            "",
        )
        .unwrap();
        assert!(!engine.check_network_request(&excluded).should_block());

        let other_source = Request::new(
            "https://example.com/a.js",
            "https://other.com",
            "script",
            "",
        )
        .unwrap();
        assert!(!engine.check_network_request(&other_source).should_block());
    }

    fn to_req(url: &str, source: &str, request_type: &str) -> Request {
        Request::new(url, source, request_type, "").unwrap()
    }

    /// Exclude-only `$to=~` narrowed by `$from=`, taken from `data/brave/brave-main-list.txt`.
    #[test]
    fn to_option_exclude_with_from_real_world_rules() {
        let engine = Engine::new_with_list_text(
            [
                // `.gif` has no usable pattern token: indexed by `$from=`.
                ".gif$from=thapcam.net,to=~thesports.com",
                // Several `$from=` domains.
                "*$frame,3p,from=jacquieetmicheltv.net|jacquieetmicheltv2.net,to=~my18pass.com",
                // Exclude-only `$to=~` without any index key is dropped.
                r"/^https:\/\/[a-z0-9-]{7,}\.[a-z]{3,6}\/assets\/script$/$script,1p,match-case,to=~edu|~gov",
            ]
            .join("\n"),
        );
        let blocked = |url: &str, source: &str, ty: &str| {
            engine
                .check_network_request(&to_req(url, source, ty))
                .should_block()
        };

        assert!(blocked(
            "https://cdn.example.com/a.gif",
            "https://thapcam.net/",
            "image"
        ));
        assert!(!blocked(
            "https://thesports.com/a.gif",
            "https://thapcam.net/",
            "image"
        ));
        // `$to=` matches subdomains of the listed host.
        assert!(!blocked(
            "https://img.thesports.com/a.gif",
            "https://thapcam.net/",
            "image"
        ));
        assert!(!blocked(
            "https://cdn.example.com/a.gif",
            "https://other.net/",
            "image"
        ));

        for source in [
            "https://jacquieetmicheltv.net/",
            "https://jacquieetmicheltv2.net/",
        ] {
            assert!(blocked(
                "https://cdn.example.com/embed",
                source,
                "sub_frame"
            ));
            assert!(!blocked("https://my18pass.com/embed", source, "sub_frame"));
            assert!(!blocked(
                "https://www.my18pass.com/embed",
                source,
                "sub_frame"
            ));
        }
        assert!(!blocked(
            "https://cdn.example.com/embed",
            "https://other.net/",
            "sub_frame"
        ));
        // `3p` is still enforced.
        assert!(!blocked(
            "https://www.jacquieetmicheltv.net/embed",
            "https://jacquieetmicheltv.net/",
            "sub_frame"
        ));

        assert!(!blocked(
            "https://abcdefgh.com/assets/script",
            "https://abcdefgh.com/",
            "script"
        ));
    }

    /// `$to=` with `$redirect` / `$redirect-rule` (brave-main-list.txt, tunein.com rules).
    #[test]
    fn to_option_redirect_real_world_rules() {
        let mut engine = Engine::new_with_list_text(
            [
                "*$xhr,redirect-rule=noop.txt,to=~pagead2.googlesyndication.com,from=tunein.com",
                "*$xhr,redirect=noop.txt,from=tunein.com,to=event.platform.tunein.com",
                "||pagead2.googlesyndication.com^",
                "||ads.example^",
            ]
            .join("\n"),
        );
        let mut resources = [Resource::simple("nooptext", MimeType::TextPlain, "")];
        resources[0].aliases.push("noop.txt".to_string());
        engine.use_resources(resources);

        let source = "https://tunein.com/";
        let redirect = Some("data:text/plain;base64,".to_owned());

        for url in [
            "https://event.platform.tunein.com/a",
            "https://eu.event.platform.tunein.com/a",
        ] {
            let result = engine.check_network_request(&to_req(url, source, "xhr"));
            assert!(result.should_block(), "{url}");
            assert_eq!(result.redirect, redirect, "{url}");
        }

        // Parent of the listed host is not matched.
        let result =
            engine.check_network_request(&to_req("https://platform.tunein.com/a", source, "xhr"));
        assert!(!result.should_block());

        // `redirect-rule` applies to requests blocked by other rules, except excluded hosts.
        let result = engine.check_network_request(&to_req("https://ads.example/a", source, "xhr"));
        assert!(result.should_block());
        assert_eq!(result.redirect, redirect);

        let result = engine.check_network_request(&to_req(
            "https://pagead2.googlesyndication.com/a",
            source,
            "xhr",
        ));
        assert!(result.should_block());
        assert_eq!(result.redirect, None);

        // `$from=` still restricts.
        let result = engine.check_network_request(&to_req(
            "https://ads.example/a",
            "https://other.com/",
            "xhr",
        ));
        assert!(result.should_block());
        assert_eq!(result.redirect, None);
    }

    /// `$to=` on exception and `$important` rules (brave-main-list.txt, smartech rules).
    #[test]
    fn to_option_exception_and_important_real_world_rules() {
        let mut engine = Engine::new_with_list_text(
            [
                "@@/smartechclient.js^$script,to=netcore.co.in|netcoresmartech.com,from=hdfcfund.com",
                "/smartechclient.js^$script,to=netcore.co.in|netcoresmartech.com,redirect=noopjs,from=hdfcfund.com,important",
                "/tracker.js^$script",
                "@@/tracker.js^$script,to=cdn.example.com",
            ]
            .join("\n"),
        );
        engine.use_resources([Resource::simple(
            "noopjs",
            MimeType::ApplicationJavascript,
            "(function() {})();",
        )]);

        let source = "https://www.hdfcfund.com/";
        let result = engine.check_network_request(&to_req(
            "https://sdk.netcoresmartech.com/smartechclient.js",
            source,
            "script",
        ));
        assert!(result.should_block(), "important wins over the exception");
        assert!(result.important);
        assert!(result.redirect.is_some());

        for (url, source) in [
            ("https://sdk.example.org/smartechclient.js", source),
            (
                "https://sdk.netcoresmartech.com/smartechclient.js",
                "https://other.com/",
            ),
        ] {
            let result = engine.check_network_request(&to_req(url, source, "script"));
            assert!(!result.should_block(), "{url} from {source}");
        }

        // An exception limited by `$to=` only excepts the listed hosts and their subdomains.
        let page = "https://page.example/";
        for (url, expected_block) in [
            ("https://evil.example/tracker.js", true),
            ("https://cdn.example.org/tracker.js", true),
            ("https://cdn.example.com/tracker.js", false),
            ("https://x.cdn.example.com/tracker.js", false),
            ("https://example.com/tracker.js", true),
            ("https://notcdn.example.com/tracker.js", true),
        ] {
            let result = engine.check_network_request(&to_req(url, page, "script"));
            assert_eq!(result.should_block(), expected_block, "{url}");
            assert_eq!(result.exception.is_some(), !expected_block, "{url}");
        }
    }

    /// `@@||asd.$generichide,to=...` from brave-main-list.txt.
    #[test]
    fn to_option_generichide_exception_real_world_rule() {
        let engine =
            Engine::new_with_list_text("@@||asd.$generichide,to=asd.homes|asd.ink|asd.pics");
        for (url, expected) in [
            ("https://asd.homes/", true),
            ("https://www.asd.homes/page", true),
            ("https://asd.ink/page", true),
            ("https://asd.pics/", true),
            ("https://asd.example/", false),
            ("https://example.homes/", false),
        ] {
            assert_eq!(
                engine.url_cosmetic_resources(url).generichide,
                expected,
                "{url}"
            );
        }
    }

    /// `$csp` is a modifier filter stored in its own list; `$to=` must work there too.
    #[test]
    fn to_option_csp_filter() {
        // `~dood.*` is an entity, which network filters do not support, but it must not break
        // the other entries.
        let engine = Engine::new_with_list_text(
            "*$csp=script-src 'none',from=456movie.com|animefever.cc,to=~youtube.com|~dood.*",
        );
        let csp =
            |url: &str, source: &str| engine.get_csp_directives(&to_req(url, source, "sub_frame"));

        for source in ["https://456movie.com/", "https://animefever.cc/"] {
            assert_eq!(
                csp("https://embed.example/x", source),
                Some("script-src 'none'".to_owned())
            );
            assert_eq!(csp("https://youtube.com/embed", source), None);
            assert_eq!(csp("https://www.youtube.com/embed", source), None);
        }
        assert_eq!(csp("https://embed.example/x", "https://other.com/"), None);
    }

    /// `$badfilter` has to disable only the filter it really duplicates.
    #[test]
    fn to_option_badfilter() {
        let engine = Engine::new_with_list_text(
            [
                "*$script,to=a.example,from=x.com",
                "*$script,to=b.example,from=x.com",
                // Options in a different order and a `$from=` rule with the same host value.
                "*$script,from=x.com,to=a.example,badfilter",
                "*$script,from=a.example",
                "*$script,to=a.example,badfilter",
            ]
            .join("\n"),
        );
        let blocked = |url: &str, source: &str| {
            engine
                .check_network_request(&to_req(url, source, "script"))
                .should_block()
        };
        assert!(!blocked("https://a.example/x.js", "https://x.com/"));
        assert!(blocked("https://b.example/x.js", "https://x.com/"));
        // `$to=a.example` badfilter must not remove `$from=a.example`.
        assert!(blocked("https://c.example/x.js", "https://a.example/"));
    }

    /// Filters that differ only in `$to=` must neither be fused by the optimizer nor share
    /// buckets.
    #[test]
    fn to_option_filters_are_not_fused() {
        use crate::filters::network::NetworkFilter;
        let filters = vec![
            NetworkFilter::parse("ad$to=a.example", true, Default::default()).unwrap(),
            NetworkFilter::parse("banner$to=b.example", true, Default::default()).unwrap(),
            NetworkFilter::parse("ad", true, Default::default()).unwrap(),
        ];
        let engine = Engine::new_with_parsed_rules(filters[..2].to_vec(), vec![]);
        for (url, expected) in [
            ("https://a.example/xad", true),
            ("https://a.example/banner", false),
            ("https://b.example/banner", true),
            ("https://b.example/xad", false),
            ("https://c.example/xad", false),
        ] {
            assert_eq!(
                engine
                    .check_network_request(&to_req(url, "https://page.example/", "script"))
                    .should_block(),
                expected,
                "{url}"
            );
        }
        // A plain filter next to the `$to=` ones keeps matching everywhere.
        let engine = Engine::new_with_parsed_rules(filters, vec![]);
        assert!(
            engine
                .check_network_request(&to_req(
                    "https://c.example/xad",
                    "https://page.example/",
                    "script"
                ))
                .should_block()
        );
    }

    /// Returns the `$to=` hosts (included, excluded) of a network rule, and the first plain
    /// `$from=`/`$domain=` host, if any. Only plain hostnames are returned.
    fn extract_to_hosts(line: &str) -> Option<(Vec<&str>, Vec<&str>, Option<&str>)> {
        const COSMETIC: [&str; 5] = ["##", "#@#", "#?#", "#$#", "#%#"];
        if line.starts_with('!') || COSMETIC.iter().any(|c| line.contains(c)) {
            return None;
        }
        let to_pos = [line.rfind(",to="), line.rfind("$to=")]
            .into_iter()
            .flatten()
            .max()?;
        let is_plain_host = |h: &&str| h.contains('.') && !h.contains(['*', '/', ' ']);
        let value = line[to_pos + ",to=".len()..].split(',').next()?;
        let included = value
            .split('|')
            .filter(|h| !h.starts_with('~'))
            .filter(is_plain_host)
            .collect();
        let excluded = value
            .split('|')
            .filter_map(|h| h.strip_prefix('~'))
            .filter(is_plain_host)
            .collect();

        let from = [",from=", "$from=", ",domain=", "$domain="]
            .into_iter()
            .filter_map(|key| line.find(key).map(|pos| &line[pos + key.len()..]))
            .filter_map(|rest| rest.split(',').next())
            .flat_map(|value| value.split('|'))
            .find(|h| !h.starts_with('~') && is_plain_host(h));
        Some((included, excluded, from))
    }

    /// Builds an engine from every `$to=` rule of the Brave list and compares it against
    /// (a) every rule evaluated on its own and (b) a serialization round trip. This covers the
    /// token selection, `$to=` buckets, builder skipping and flatbuffer storage on real data.
    #[test]
    fn to_option_brave_list_matches_each_rule_alone() {
        let list = rules_from_lists(["data/brave/brave-main-list.txt"]);
        let rules: Vec<&str> = list
            .lines()
            .filter(|line| line.contains("to="))
            .filter(|line| extract_to_hosts(line).is_some())
            .collect();
        assert!(rules.len() > 100, "got only {} rules", rules.len());

        // Requests: every host named by a rule, the rule's own `$from=` (or a neutral page).
        let mut cases = std::collections::BTreeSet::new();
        for rule in &rules {
            let (included, excluded, from) = extract_to_hosts(rule).unwrap();
            let source = format!("https://{}/", from.unwrap_or("page.example"));
            let hosts = included
                .iter()
                .chain(excluded.iter())
                .map(|h| h.to_string())
                .chain(["neutral.example".to_string()]);
            for host in hosts {
                cases.insert((format!("https://{host}/a/b.js?x=1"), source.clone()));
                cases.insert((format!("https://sub.{host}/a/b.js?x=1"), source.clone()));
            }
        }
        let requests: Vec<Request> = cases
            .iter()
            .flat_map(|(url, source)| {
                ["script", "xhr", "document"]
                    .into_iter()
                    .filter_map(|ty| Request::new(url, source, ty, "").ok())
            })
            .collect();

        let engine = Engine::new_with_list_text(rules.join("\n"));
        // Exceptions never produce `filter`, so only blocking rules are evaluated alone.
        let single_engines: Vec<Engine> = rules
            .iter()
            .filter(|rule| !rule.starts_with("@@"))
            .map(|rule| Engine::new_with_list_text(*rule))
            .collect();

        let mut round_trip = Engine::default();
        let engine_for_dat = Engine::new_with_list_text(rules.join("\n"));
        let dat = engine_for_dat.serialize();
        round_trip
            .deserialize(&dat)
            .expect("deserialization failed");

        let mut blocked = 0;
        for request in &requests {
            let result = engine.check_network_request(request);
            let alone = single_engines
                .iter()
                .any(|e| e.check_network_request(request).filter.is_some());
            assert_eq!(
                result.filter.is_some(),
                alone,
                "engine and per-rule results differ for {}",
                request.url
            );

            let result_rt = round_trip.check_network_request(request);
            assert_eq!(result.filter.is_some(), result_rt.filter.is_some());
            assert_eq!(result.exception.is_some(), result_rt.exception.is_some());
            assert_eq!(result.important, result_rt.important);
            assert_eq!(result.redirect, result_rt.redirect);

            blocked += usize::from(result.filter.is_some());
        }
        assert!(
            blocked >= 10,
            "too few matches ({blocked}) out of {} requests",
            requests.len()
        );
    }
}
