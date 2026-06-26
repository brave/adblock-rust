#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_hosts_style() {
        {
            let input = "www.malware.com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_ok());
        }
        {
            let input = "www.malware.com/virus.txt";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 www.malware.com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_ok());
        }
        {
            let input = "127.0.0.1\t\twww.malware.com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com     # replace after issue #289336 is addressed";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_ok());
        }
        {
            let input = "! Title: list.txt";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 localhost";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = ".com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = "*.com";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
        {
            let input = "www.";
            let result = parse_filter(
                input,
                true,
                ParseOptions {
                    format: FilterFormat::Hosts,
                    ..Default::default()
                },
            );
            assert!(result.is_err());
        }
    }

    #[test]
    fn adguard_cosmetic_detection() {
        {
            let input = r#"example.org$$script[data-src="banner"]"#;
            let result = parse_filter(input, true, Default::default());
            assert!(result.is_err());
        }
        {
            let input = "example.org##+js(set-local-storage-item, Test, $$remove$$)";
            let result = parse_filter(input, true, Default::default());
            assert!(result.is_ok());
        }
        {
            let input = "[$app=org.example.app]example.com##.textad";
            let result = parse_filter(input, true, Default::default());
            assert!(result.is_err());
        }
        {
            let input = r#"[$domain=/^i\[a-z\]*\.strmrdr\[a-z\]+\..*/]##+js(set-constant, adscfg.enabled, false)"#;
            let result = parse_filter(input, true, Default::default());
            assert!(result.is_err());
        }
    }

    #[test]
    fn parse_filter_failed_fuzz_1() {
        let input = "Ѥ";
        let result = parse_filter(input, true, Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_2() {
        assert!(parse_filter(r#"###\\\00DB \008D"#, true, Default::default()).is_ok());
        assert!(parse_filter(r#"###\Û"#, true, Default::default()).is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_3() {
        let input = "||$3p=/";
        let result = parse_filter(input, true, Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_4() {
        // \\##+js(,\xdd\x8d
        let parsed = parse_filter(
            &String::from_utf8(vec![92, 35, 35, 43, 106, 115, 40, 44, 221, 141]).unwrap(),
            true,
            Default::default(),
        );
        #[cfg(feature = "css-validation")]
        assert!(parsed.is_err());
        #[cfg(not(feature = "css-validation"))]
        assert!(parsed.is_ok());
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn parse_filter_opening_comment() {
        assert!(parse_filter("##input,input/*", true, Default::default(),).is_err());
    }

    #[test]
    fn test_parse_expires_interval() {
        assert_eq!(ExpiresInterval::try_from("0 hour"), Err(()));
        assert_eq!(ExpiresInterval::try_from("0 hours"), Err(()));
        assert_eq!(
            ExpiresInterval::try_from("1 hour"),
            Ok(ExpiresInterval::Hours(1))
        );
        assert_eq!(
            ExpiresInterval::try_from("1 hours"),
            Ok(ExpiresInterval::Hours(1))
        );
        assert_eq!(
            ExpiresInterval::try_from("2 hours"),
            Ok(ExpiresInterval::Hours(2))
        );
        assert_eq!(
            ExpiresInterval::try_from("2 hour"),
            Ok(ExpiresInterval::Hours(2))
        );
        assert_eq!(ExpiresInterval::try_from("3.5 hours"), Err(()));
        assert_eq!(
            ExpiresInterval::try_from("336 hours"),
            Ok(ExpiresInterval::Hours(336))
        );
        assert_eq!(ExpiresInterval::try_from("337 hours"), Err(()));

        assert_eq!(ExpiresInterval::try_from("0 day"), Err(()));
        assert_eq!(ExpiresInterval::try_from("0 days"), Err(()));
        assert_eq!(
            ExpiresInterval::try_from("1 day"),
            Ok(ExpiresInterval::Days(1))
        );
        assert_eq!(
            ExpiresInterval::try_from("1 days"),
            Ok(ExpiresInterval::Days(1))
        );
        assert_eq!(
            ExpiresInterval::try_from("2 days"),
            Ok(ExpiresInterval::Days(2))
        );
        assert_eq!(
            ExpiresInterval::try_from("2 day"),
            Ok(ExpiresInterval::Days(2))
        );
        assert_eq!(ExpiresInterval::try_from("3.5 days"), Err(()));
        assert_eq!(
            ExpiresInterval::try_from("14 days"),
            Ok(ExpiresInterval::Days(14))
        );
        assert_eq!(ExpiresInterval::try_from("15 days"), Err(()));

        assert_eq!(ExpiresInterval::try_from("-5 hours"), Err(()));
        assert_eq!(ExpiresInterval::try_from("+5 hours"), Err(()));

        assert_eq!(
            ExpiresInterval::try_from("2 days (update frequency)"),
            Ok(ExpiresInterval::Days(2))
        );
        assert_eq!(
            ExpiresInterval::try_from("2 hours (update frequency)"),
            Ok(ExpiresInterval::Hours(2))
        );
    }

    #[test]
    fn test_parsing_list_metadata() {
        let list = [
            "[Adblock Plus 2.0]",
            "! Title: 0131 Block List",
            "! Homepage: https://austinhuang.me/0131-block-list",
            "! Licence: https://creativecommons.org/licenses/by-sa/4.0/",
            "! Expires: 7 days",
            "! Version: 20220411",
            "",
            "! => https://austinhuang.me/0131-block-list/list.txt",
        ];

        let mut filter_set = FilterSet::new(false);
        let metadata = filter_set.add_filters(list, ParseOptions::default());

        assert_eq!(metadata.title, Some("0131 Block List".to_string()));
        assert_eq!(
            metadata.homepage,
            Some("https://austinhuang.me/0131-block-list".to_string())
        );
        assert_eq!(metadata.expires, Some(ExpiresInterval::Days(7)));
        assert_eq!(metadata.redirect, None);
    }

    #[test]
    /// Some lists are formatted in unusual ways. This example has a version string with
    /// non-numeric characters and an `Expires` field with extra information trailing afterwards.
    /// Valid fields should still be recognized and parsed accordingly.
    fn test_parsing_list_best_effort() {
        let list = [
            "[Adblock Plus 2]",
            "!-----------------------------------",
            "!             ABOUT",
            "!-----------------------------------",
            "! Version: 1.2.0.0",
            "! Title: ABPVN Advanced",
            "! Last modified: 09/03/2021",
            "! Expires: 7 days (update frequency)",
            "! Homepage: https://www.haopro.net/",
        ];

        let mut filter_set = FilterSet::new(false);
        let metadata = filter_set.add_filters(list, ParseOptions::default());

        assert_eq!(metadata.title, Some("ABPVN Advanced".to_string()));
        assert_eq!(
            metadata.homepage,
            Some("https://www.haopro.net/".to_string())
        );
        assert_eq!(metadata.expires, Some(ExpiresInterval::Days(7)));
        assert_eq!(metadata.redirect, None);
    }

    #[test]
    fn test_read_metadata() {
        {
            let list = r##"! Title: uBlock₀ filters – Annoyances
! Description: Filters optimized for uBlock Origin, to be used with Fanboy's
!              and/or Adguard's "Annoyances" list(s)
! Expires: 4 days
! Last modified: %timestamp%
! License: https://github.com/uBlockOrigin/uAssets/blob/master/LICENSE
! Homepage: https://github.com/uBlockOrigin/uAssets
! Forums: https://github.com/uBlockOrigin/uAssets/issues"##;
            let metadata = read_list_metadata(list);

            assert_eq!(
                metadata.title,
                Some("uBlock₀ filters – Annoyances".to_string())
            );
            assert_eq!(
                metadata.homepage,
                Some("https://github.com/uBlockOrigin/uAssets".to_string())
            );
            assert_eq!(metadata.expires, Some(ExpiresInterval::Days(4)));
            assert_eq!(metadata.redirect, None);
        }
        {
            let list = r##"[uBlock Origin]
! Title: PersianBlocker
! Description: سرانجام، یک لیست بهینه و گسترده برای مسدودسازی تبلیغ ها و ردیاب ها در سایت های پارسی زبان!
! Expires: 2 days
! Last modified: 2022-12-11
! Homepage: https://github.com/MasterKia/PersianBlocker
! License: AGPLv3 (https://github.com/MasterKia/PersianBlocker/blob/main/LICENSE)

! مشکل/پیشنهاد: https://github.com/MasterKia/PersianBlocker/issues
! مشارکت: https://github.com/MasterKia/PersianBlocker/pulls

!  لیستی برای برگرداندن آزادی کاربران، چون هر کاربر این آزادی را دارد که چه چیزی وارد مرورگرش می‌شود و چه چیزی وارد نمی‌شود
!-------------------------v Experimental Generic Filters v-----------------------!
! applicationha.com, androidgozar.com, downloadkral.com, gold-team.org, iranecar.com, icoff.ee, koolakmag.ir,
!! mybia4music.com, my-film.pw, pedal.ir, vgdl.ir, sakhamusic.ir
/wp-admin/admin-ajax.php?postviews_id=$xhr
"##;
            let metadata = read_list_metadata(list);

            assert_eq!(metadata.title, Some("PersianBlocker".to_string()));
            assert_eq!(
                metadata.homepage,
                Some("https://github.com/MasterKia/PersianBlocker".to_string())
            );
            assert_eq!(metadata.expires, Some(ExpiresInterval::Days(2)));
            assert_eq!(metadata.redirect, None);
        }
    }

    #[test]
    fn parse_cosmetic_variants() {
        {
            let input = "example.com##.selector";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Ok(ParsedFilter::Cosmetic(..))));
        }
        {
            let input = "9gag.com#?#article:-abp-has(.promoted)";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Ok(ParsedFilter::Cosmetic(..))));
        }
        #[cfg(feature = "css-validation")]
        {
            let input = "sportowefakty.wp.pl#@?#body > [class]:not([id]):matches-css(position: fixed):matches-css(top: 0px)";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Ok(ParsedFilter::Cosmetic(..))));
        }
        {
            let input =
                r#"odkrywamyzakryte.com#%#//scriptlet("abort-on-property-read", "sc_adv_out")"#;
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(
                result,
                Err(FilterParseError::Cosmetic(
                    CosmeticFilterError::UnsupportedSyntax
                ))
            ));
        }
        {
            let input = "bikeradar.com,spiegel.de#@%#!function(){function b(){}function a(a){return{get:function(){return a},set:b}}function c(a)";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(
                result,
                Err(FilterParseError::Cosmetic(
                    CosmeticFilterError::UnsupportedSyntax
                ))
            ));
        }
        {
            let input = "nczas.com#$#.adsbygoogle { position: absolute!important; left: -3000px!important; }";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(
                result,
                Err(FilterParseError::Cosmetic(
                    CosmeticFilterError::UnsupportedSyntax
                ))
            ));
        }
        {
            let input =
                "kurnik.pl#@$#.adsbygoogle { height: 1px !important; width: 1px !important; }";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(
                result,
                Err(FilterParseError::Cosmetic(
                    CosmeticFilterError::UnsupportedSyntax
                ))
            ));
        }
    }

    #[test]
    fn incremental_compilation_matches_vec_path() {
        use crate::request::Request;
        use crate::Engine;
        use seahash::hash;

        let rules = [
            "||ads.example.com^",
            "||tracker.example.com^",
            "||ads.example.com^$badfilter",
            "example.org##.ad",
            "@@||allowed.example.com^",
        ];

        let (network_filters, cosmetic_filters) = parse_filters(rules, false, Default::default());
        let engine_from_vec = Engine::from_filter_set(
            FilterSet::new_with_rules(network_filters, cosmetic_filters, false),
            true,
        );

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(rules, Default::default());
        let engine_incremental = Engine::from_filter_set(filter_set, true);

        let requests = [
            (
                "https://ads.example.com/track.js",
                "https://publisher.com",
                "script",
            ),
            (
                "https://tracker.example.com/pixel",
                "https://publisher.com",
                "image",
            ),
            (
                "https://allowed.example.com/script.js",
                "https://publisher.com",
                "script",
            ),
        ];
        for (url, source, cpt) in requests {
            let request = Request::new(url, source, cpt).unwrap();
            assert_eq!(
                engine_from_vec.check_network_request(&request).matched,
                engine_incremental.check_network_request(&request).matched,
                "mismatch for {url}"
            );
        }

        assert_eq!(
            hash(&engine_from_vec.serialize()),
            hash(&engine_incremental.serialize()),
            "serialized engines should be identical"
        );
    }

    #[test]
    fn filter_set_clone_preserves_blocking_after_engine_construction() {
        use crate::request::Request;
        use crate::Engine;

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(["||blocked.example.com^"], Default::default());
        let cloned = filter_set.clone();
        let _engine = Engine::from_filter_set(filter_set, true);

        let mut extended = cloned;
        extended
            .add_filter("||also-blocked.example.com^", Default::default())
            .unwrap();
        let engine = Engine::from_filter_set(extended, true);

        let blocked = Request::new(
            "https://also-blocked.example.com/a.js",
            "https://page.com",
            "script",
        )
        .unwrap();
        assert!(engine.check_network_request(&blocked).matched);
    }

    #[test]
    fn network_only_skips_cosmetic_rules() {
        let rules = [
            "||ads.example.com^",
            "example.org##.ad-banner",
            "example.net#@#.promo",
        ];

        let mut all = FilterSet::new(false);
        all.add_filters(rules, ParseOptions::default());
        assert_eq!(all.cosmetic_filters.len(), 2);

        let mut network_only = FilterSet::new(false);
        network_only.add_filters(
            rules,
            ParseOptions {
                rule_types: RuleTypes::NetworkOnly,
                ..Default::default()
            },
        );
        assert!(network_only.cosmetic_filters.is_empty());
        assert_eq!(
            network_only
                .compilation
                .as_ref()
                .unwrap()
                .network_rules
                .rule_count(),
            1
        );
    }

    #[test]
    fn network_only_preserves_network_blocking() {
        use crate::request::Request;
        use crate::Engine;

        let rules = ["||ads.example.com^", "example.org##.ad-banner"];

        let engine_all = Engine::from_rules(rules, Default::default());
        let engine_network = Engine::from_rules_network(rules, Default::default());

        let request = Request::new(
            "https://ads.example.com/track.js",
            "https://publisher.com",
            "script",
        )
        .unwrap();

        assert_eq!(
            engine_all.check_network_request(&request).matched,
            engine_network.check_network_request(&request).matched,
        );
    }

    #[test]
    fn brave_streaming_matches_vec_path() {
        use crate::lists::parse_filters;
        use crate::test_utils::rules_from_lists;
        use crate::Engine;
        use seahash::hash;

        let rules: Vec<String> = rules_from_lists(&["data/brave/brave-main-list.txt"]).collect();
        let (network_filters, cosmetic_filters) =
            parse_filters(rules.clone(), false, Default::default());
        let engine_from_vec = Engine::from_filter_set(
            FilterSet::new_with_rules(network_filters, cosmetic_filters, false),
            true,
        );

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(rules, Default::default());
        let engine_streaming = Engine::from_filter_set(filter_set, true);

        assert_eq!(
            hash(&engine_from_vec.serialize()),
            hash(&engine_streaming.serialize()),
            "brave list vec and streaming paths should match"
        );
    }

    #[test]
    fn add_filter_list_incremental_matches_add_filters() {
        use crate::Engine;
        use seahash::hash;

        let rules = [
            "||ads.example.com^",
            "||tracker.example.com^",
            "example.org##.ad",
            "@@||allowed.example.com^",
        ];
        let list_text = rules.join("\n");

        let mut from_filters = FilterSet::new(false);
        from_filters.add_filters(rules, Default::default());

        let mut from_list = FilterSet::new(false);
        from_list.add_filter_list(&list_text, Default::default());

        assert_eq!(
            from_filters.cosmetic_filters.len(),
            from_list.cosmetic_filters.len()
        );
        assert_eq!(
            from_filters
                .compilation
                .as_ref()
                .unwrap()
                .network_rules
                .rule_count(),
            from_list
                .compilation
                .as_ref()
                .unwrap()
                .network_rules
                .rule_count()
        );

        let engine_filters = Engine::from_filter_set(from_filters, true);
        let engine_list = Engine::from_filter_set(from_list, true);

        assert_eq!(
            hash(&engine_filters.serialize()),
            hash(&engine_list.serialize()),
            "add_filter_list and add_filters should produce identical engines"
        );
    }

    #[test]
    fn add_filter_list_badfilter_prescan_skips_invalidated_rule() {
        use crate::request::Request;
        use crate::Engine;

        let list = "! Title\n||ads.example.com^\n||ads.example.com^$badfilter\n";
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(list, Default::default());

        let engine = Engine::from_filter_set(filter_set, true);
        let request = Request::new(
            "https://ads.example.com/track.js",
            "https://publisher.com",
            "script",
        )
        .unwrap();

        assert!(
            !engine.check_network_request(&request).matched,
            "badfilter prescan should invalidate the earlier blocking rule"
        );
    }

    #[test]
    fn add_filters_badfilter_removes_incrementally_routed_rule() {
        use crate::request::Request;
        use crate::Engine;

        let rules = ["||ads.example.com^", "||ads.example.com^$badfilter"];

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(rules, Default::default());

        let engine = Engine::from_filter_set(filter_set, true);
        let request = Request::new(
            "https://ads.example.com/track.js",
            "https://publisher.com",
            "script",
        )
        .unwrap();

        assert!(
            !engine.check_network_request(&request).matched,
            "retroactive badfilter should remove the routed rule"
        );
    }

    #[test]
    fn filter_set_clone_preserves_rules_after_filter_list_engine() {
        use crate::request::Request;
        use crate::Engine;

        let list = "||blocked.example.com^\n";
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(list, Default::default());
        let cloned = filter_set.clone();
        let _engine = Engine::from_filter_set(filter_set, true);

        let mut extended = cloned;
        extended
            .add_filter("||also-blocked.example.com^", Default::default())
            .unwrap();
        let engine = Engine::from_filter_set(extended, true);

        let blocked = Request::new(
            "https://also-blocked.example.com/a.js",
            "https://page.com",
            "script",
        )
        .unwrap();
        assert!(engine.check_network_request(&blocked).matched);
    }

    #[test]
    fn incremental_compilation_without_optimize_matches_vec_path() {
        use crate::Engine;
        use seahash::hash;

        let rules = [
            "||ads.example.com^",
            "||tracker.example.com^",
            "example.org##.ad",
        ];

        let (network_filters, cosmetic_filters) = parse_filters(rules, false, Default::default());
        let engine_from_vec = Engine::from_filter_set(
            FilterSet::new_with_rules(network_filters, cosmetic_filters, false),
            false,
        );

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(rules, Default::default());
        let engine_incremental = Engine::from_filter_set(filter_set, false);

        assert_eq!(
            hash(&engine_from_vec.serialize()),
            hash(&engine_incremental.serialize()),
            "optimize=false should match between vec and incremental paths"
        );
    }

    #[test]
    fn network_rules_routes_filters_to_specialized_lists() {
        use crate::filters::fb_network_builder::NetworkFilterListId;

        let rules = [
            "||blocked.example.com^",
            "@@||allowed.example.com^",
            "||important.example.com^$important",
            "@@||gh.example.com^$generichide",
            "||csp.example.com^$csp=script-src 'none'",
            "||tagged.example.com^$tag=foo",
            "||remove.example.com^$removeparam=utm",
            "||redirect.example.com^$redirect-rule=noop.js",
        ];

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(rules, Default::default());
        let network_rules = &filter_set.compilation.as_ref().unwrap().network_rules;

        assert_eq!(network_rules.rule_count(), 8);
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Filters),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Exceptions),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Importants),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::GenericHide),
            1
        );
        assert_eq!(network_rules.list_rule_count(NetworkFilterListId::Csp), 1);
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::TaggedFiltersAll),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::RemoveParam),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Redirects),
            1
        );
    }

    #[test]
    fn redirect_with_also_block_routes_to_filters_and_redirects() {
        use crate::filters::fb_network_builder::NetworkFilterListId;

        let mut filter_set = FilterSet::new(false);
        filter_set.add_filters(
            ["||redirect.example.com^$redirect=noop.js"],
            Default::default(),
        );
        let network_rules = &filter_set.compilation.as_ref().unwrap().network_rules;

        assert_eq!(network_rules.rule_count(), 2);
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Redirects),
            1
        );
        assert_eq!(
            network_rules.list_rule_count(NetworkFilterListId::Filters),
            1
        );
    }

    #[test]
    fn debug_filter_set_keeps_vec_instead_of_compilation() {
        let mut filter_set = FilterSet::new(true);
        filter_set.add_filters(["||debug.example.com^"], Default::default());

        assert!(filter_set.compilation.is_none());
        assert_eq!(filter_set.network_filters.len(), 1);
    }

    #[test]
    fn add_filter_list_reads_metadata_without_scanning_whole_list() {
        let list = "! Title: Example\n! Expires: 2 days\n||ads.example.com^\n";
        let mut filter_set = FilterSet::new(false);
        let metadata = filter_set.add_filter_list(list, Default::default());

        assert_eq!(metadata.title, Some("Example".to_string()));
        assert_eq!(metadata.expires, Some(ExpiresInterval::Days(2)));
    }

    #[test]
    fn network_only_add_filter_list_skips_cosmetic_rules() {
        let list = "||ads.example.com^\nexample.org##.ad-banner\n";
        let mut filter_set = FilterSet::new(false);
        filter_set.add_filter_list(
            list,
            ParseOptions {
                rule_types: RuleTypes::NetworkOnly,
                ..Default::default()
            },
        );

        assert!(filter_set.cosmetic_filters.is_empty());
        assert_eq!(
            filter_set
                .compilation
                .as_ref()
                .unwrap()
                .network_rules
                .rule_count(),
            1
        );
    }
}
