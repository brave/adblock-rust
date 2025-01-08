#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn parse_hosts_style() {
        {
            let input = "www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "www.malware.com/virus.txt";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "127.0.0.1\t\twww.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com     # replace after issue #289336 is addressed";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "! Title: list.txt";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 localhost";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = ".com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "*.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "www.";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
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
        assert!(parse_filter(
            "##input,input/*",
            true,
            Default::default(),
        ).is_err());
    }

    #[test]
    fn test_parse_expires_interval() {
        assert_eq!(ExpiresInterval::try_from("0 hour"), Err(()));
        assert_eq!(ExpiresInterval::try_from("0 hours"), Err(()));
        assert_eq!(ExpiresInterval::try_from("1 hour"), Ok(ExpiresInterval::Hours(1)));
        assert_eq!(ExpiresInterval::try_from("1 hours"), Ok(ExpiresInterval::Hours(1)));
        assert_eq!(ExpiresInterval::try_from("2 hours"), Ok(ExpiresInterval::Hours(2)));
        assert_eq!(ExpiresInterval::try_from("2 hour"), Ok(ExpiresInterval::Hours(2)));
        assert_eq!(ExpiresInterval::try_from("3.5 hours"), Err(()));
        assert_eq!(ExpiresInterval::try_from("336 hours"), Ok(ExpiresInterval::Hours(336)));
        assert_eq!(ExpiresInterval::try_from("337 hours"), Err(()));

        assert_eq!(ExpiresInterval::try_from("0 day"), Err(()));
        assert_eq!(ExpiresInterval::try_from("0 days"), Err(()));
        assert_eq!(ExpiresInterval::try_from("1 day"), Ok(ExpiresInterval::Days(1)));
        assert_eq!(ExpiresInterval::try_from("1 days"), Ok(ExpiresInterval::Days(1)));
        assert_eq!(ExpiresInterval::try_from("2 days"), Ok(ExpiresInterval::Days(2)));
        assert_eq!(ExpiresInterval::try_from("2 day"), Ok(ExpiresInterval::Days(2)));
        assert_eq!(ExpiresInterval::try_from("3.5 days"), Err(()));
        assert_eq!(ExpiresInterval::try_from("14 days"), Ok(ExpiresInterval::Days(14)));
        assert_eq!(ExpiresInterval::try_from("15 days"), Err(()));

        assert_eq!(ExpiresInterval::try_from("-5 hours"), Err(()));
        assert_eq!(ExpiresInterval::try_from("+5 hours"), Err(()));

        assert_eq!(ExpiresInterval::try_from("2 days (update frequency)"), Ok(ExpiresInterval::Days(2)));
        assert_eq!(ExpiresInterval::try_from("2 hours (update frequency)"), Ok(ExpiresInterval::Hours(2)));
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
        assert_eq!(metadata.homepage, Some("https://austinhuang.me/0131-block-list".to_string()));
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
        assert_eq!(metadata.homepage, Some("https://www.haopro.net/".to_string()));
        assert_eq!(metadata.expires, Some(ExpiresInterval::Days(7)));
        assert_eq!(metadata.redirect, None);
    }

    #[test]
    fn test_read_metadata() {
        {
            let list =
r##"! Title: uBlock₀ filters – Annoyances
! Description: Filters optimized for uBlock Origin, to be used with Fanboy's
!              and/or Adguard's "Annoyances" list(s)
! Expires: 4 days
! Last modified: %timestamp%
! License: https://github.com/uBlockOrigin/uAssets/blob/master/LICENSE
! Homepage: https://github.com/uBlockOrigin/uAssets
! Forums: https://github.com/uBlockOrigin/uAssets/issues"##;
            let metadata = read_list_metadata(&list);

            assert_eq!(metadata.title, Some("uBlock₀ filters – Annoyances".to_string()));
            assert_eq!(metadata.homepage, Some("https://github.com/uBlockOrigin/uAssets".to_string()));
            assert_eq!(metadata.expires, Some(ExpiresInterval::Days(4)));
            assert_eq!(metadata.redirect, None);
        }
        {
            let list =
r##"[uBlock Origin]
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
            let metadata = read_list_metadata(&list);

            assert_eq!(metadata.title, Some("PersianBlocker".to_string()));
            assert_eq!(metadata.homepage, Some("https://github.com/MasterKia/PersianBlocker".to_string()));
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
            let input = r#"odkrywamyzakryte.com#%#//scriptlet("abort-on-property-read", "sc_adv_out")"#;
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Err(FilterParseError::Cosmetic(CosmeticFilterError::UnsupportedSyntax))));
        }
        {
            let input = "bikeradar.com,spiegel.de#@%#!function(){function b(){}function a(a){return{get:function(){return a},set:b}}function c(a)";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Err(FilterParseError::Cosmetic(CosmeticFilterError::UnsupportedSyntax))));
        }
        {
            let input = "nczas.com#$#.adsbygoogle { position: absolute!important; left: -3000px!important; }";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Err(FilterParseError::Cosmetic(CosmeticFilterError::UnsupportedSyntax))));
        }
        {
            let input = "kurnik.pl#@$#.adsbygoogle { height: 1px !important; width: 1px !important; }";
            let result = parse_filter(input, true, Default::default());
            assert!(matches!(result, Err(FilterParseError::Cosmetic(CosmeticFilterError::UnsupportedSyntax))));
        }
    }
}
