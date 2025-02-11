#[cfg(test)]
mod parse_tests {
    use super::super::*;

    /// An easily modified summary of a `CosmeticFilter` rule to be used in tests.
    #[derive(Debug, PartialEq)]
    struct CosmeticFilterBreakdown {
        entities: Option<Vec<Hash>>,
        hostnames: Option<Vec<Hash>>,
        not_entities: Option<Vec<Hash>>,
        not_hostnames: Option<Vec<Hash>>,
        selector: SelectorType,
        action: Option<CosmeticFilterAction>,

        unhide: bool,
        script_inject: bool,
    }

    impl From<&CosmeticFilter> for CosmeticFilterBreakdown {
        fn from(filter: &CosmeticFilter) -> CosmeticFilterBreakdown {
            CosmeticFilterBreakdown {
                entities: filter.entities.as_ref().cloned(),
                hostnames: filter.hostnames.as_ref().cloned(),
                not_entities: filter.not_entities.as_ref().cloned(),
                not_hostnames: filter.not_hostnames.as_ref().cloned(),
                selector: SelectorType::from(filter),
                action: filter.action.as_ref().cloned(),

                unhide: filter.mask.contains(CosmeticFilterMask::UNHIDE),
                script_inject: filter.mask.contains(CosmeticFilterMask::SCRIPT_INJECT),
            }
        }
    }

    impl From<CosmeticFilter> for CosmeticFilterBreakdown {
        fn from(filter: CosmeticFilter) -> CosmeticFilterBreakdown {
            (&filter).into()
        }
    }

    impl Default for CosmeticFilterBreakdown {
        fn default() -> Self {
            CosmeticFilterBreakdown {
                entities: None,
                hostnames: None,
                not_entities: None,
                not_hostnames: None,
                selector: SelectorType::PlainCss(String::from("")),
                action: None,

                unhide: false,
                script_inject: false,
            }
        }
    }

    #[derive(Debug, PartialEq)]
    enum SelectorType {
        PlainCss(String),
        Procedural(Vec<CosmeticFilterOperator>),
    }

    impl From<&CosmeticFilter> for SelectorType {
        fn from(v: &CosmeticFilter) -> Self {
            if let Some(selector) = v.plain_css_selector() {
                Self::PlainCss(selector.to_string())
            } else {
                Self::Procedural(v.selector.clone())
            }
        }
    }

    fn parse_cf(rule: &str) -> Result<CosmeticFilter, CosmeticFilterError> {
        CosmeticFilter::parse(rule, false, Default::default())
    }

    /// Asserts that `rule` parses into a `CosmeticFilter` equivalent to the summary provided by
    /// `expected`.
    fn check_parse_result(rule: &str, expected: CosmeticFilterBreakdown) {
        let filter: CosmeticFilterBreakdown = parse_cf(rule).unwrap().into();
        assert_eq!(expected, filter);
    }

    #[test]
    fn simple_selectors() {
        check_parse_result(
            "##div.popup",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("div.popup".to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            "###selector",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("#selector".to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            "##.selector",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(".selector".to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            "##a[href=\"foo.com\"]",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("a[href=\"foo.com\"]".to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            "##[href=\"foo.com\"]",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("[href=\"foo.com\"]".to_string()),
                ..Default::default()
            },
        );
    }

    /// Produces a sorted vec of the hashes of all the given domains.
    ///
    /// For convenience, the return value is wrapped in a `Some()` to be consumed by a
    /// `CosmeticFilterBreakdown`.
    fn sort_hash_domains(domains: Vec<&str>) -> Option<Vec<Hash>> {
        let mut hashes: Vec<_> = domains.iter().map(|d| crate::utils::fast_hash(d)).collect();
        hashes.sort();
        Some(hashes)
    }

    #[test]
    fn hostnames() {
        check_parse_result(
            r#"u00p.com##div[class^="adv-box"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"div[class^="adv-box"]"#.to_string()),
                hostnames: sort_hash_domains(vec!["u00p.com"]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"distractify.com##div[class*="AdInArticle"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"div[class*="AdInArticle"]"#.to_string()),
                hostnames: sort_hash_domains(vec!["distractify.com"]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"soundtrackcollector.com,the-numbers.com##a[href^="http://affiliates.allposters.com/"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"a[href^="http://affiliates.allposters.com/"]"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec!["soundtrackcollector.com", "the-numbers.com"]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"thelocal.at,thelocal.ch,thelocal.de,thelocal.dk,thelocal.es,thelocal.fr,thelocal.it,thelocal.no,thelocal.se##div[class*="-widget"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"div[class*="-widget"]"#.to_string()),
                hostnames: sort_hash_domains(vec![
                    "thelocal.at",
                    "thelocal.ch",
                    "thelocal.de",
                    "thelocal.dk",
                    "thelocal.es",
                    "thelocal.fr",
                    "thelocal.it",
                    "thelocal.no",
                    "thelocal.se",
                ]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"base64decode.org,base64encode.org,beautifyjson.org,minifyjson.org,numgen.org,pdfmrg.com,pdfspl.com,prettifycss.com,pwdgen.org,strlength.com,strreverse.com,uglifyjs.net,urldecoder.org##div[class^="banner_"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"div[class^="banner_"]"#.to_string()),
                hostnames: sort_hash_domains(vec![
                    "base64decode.org",
                    "base64encode.org",
                    "beautifyjson.org",
                    "minifyjson.org",
                    "numgen.org",
                    "pdfmrg.com",
                    "pdfspl.com",
                    "prettifycss.com",
                    "pwdgen.org",
                    "strlength.com",
                    "strreverse.com",
                    "uglifyjs.net",
                    "urldecoder.org",
                ]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"adforum.com,alliednews.com,americustimesrecorder.com,andovertownsman.com,athensreview.com,batesvilleheraldtribune.com,bdtonline.com,channel24.pk,chickashanews.com,claremoreprogress.com,cleburnetimesreview.com,clintonherald.com,commercejournal.com,commercial-news.com,coopercrier.com,cordeledispatch.com,corsicanadailysun.com,crossville-chronicle.com,cullmantimes.com,dailyiowegian.com,dailyitem.com,daltondailycitizen.com,derrynews.com,duncanbanner.com,eagletribune.com,edmondsun.com,effinghamdailynews.com,enewscourier.com,enidnews.com,farmtalknewspaper.com,fayettetribune.com,flasharcade.com,flashgames247.com,flyergroup.com,foxsportsasia.com,gainesvilleregister.com,gloucestertimes.com,goshennews.com,greensburgdailynews.com,heraldbanner.com,heraldbulletin.com,hgazette.com,homemagonline.com,itemonline.com,jacksonvilleprogress.com,jerusalemonline.com,joplinglobe.com,journal-times.com,journalexpress.net,kexp.org,kokomotribune.com,lockportjournal.com,mankatofreepress.com,mcalesternews.com,mccrearyrecord.com,mcleansborotimesleader.com,meadvilletribune.com,meridianstar.com,mineralwellsindex.com,montgomery-herald.com,mooreamerican.com,moultrieobserver.com,muskogeephoenix.com,ncnewsonline.com,newburyportnews.com,newsaegis.com,newsandtribune.com,niagara-gazette.com,njeffersonnews.com,normantranscript.com,opposingviews.com,orangeleader.com,oskaloosa.com,ottumwacourier.com,outlookmoney.com,palestineherald.com,panews.com,paulsvalleydailydemocrat.com,pellachronicle.com,pharostribune.com,pressrepublican.com,pryordailytimes.com,randolphguide.com,record-eagle.com,register-herald.com,register-news.com,reporter.net,rockwallheraldbanner.com,roysecityheraldbanner.com,rushvillerepublican.com,salemnews.com,sentinel-echo.com,sharonherald.com,shelbyvilledailyunion.com,siteslike.com,standardmedia.co.ke,starbeacon.com,stwnewspress.com,suwanneedemocrat.com,tahlequahdailypress.com,theadanews.com,theawesomer.com,thedailystar.com,thelandonline.com,themoreheadnews.com,thesnaponline.com,tiftongazette.com,times-news.com,timesenterprise.com,timessentinel.com,timeswv.com,tonawanda-news.com,tribdem.com,tribstar.com,unionrecorder.com,valdostadailytimes.com,washtimesherald.com,waurikademocrat.com,wcoutlook.com,weatherforddemocrat.com,woodwardnews.net,wrestlinginc.com##div[style="width:300px; height:250px;"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"div[style="width:300px; height:250px;"]"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec![
                    "adforum.com",
                    "alliednews.com",
                    "americustimesrecorder.com",
                    "andovertownsman.com",
                    "athensreview.com",
                    "batesvilleheraldtribune.com",
                    "bdtonline.com",
                    "channel24.pk",
                    "chickashanews.com",
                    "claremoreprogress.com",
                    "cleburnetimesreview.com",
                    "clintonherald.com",
                    "commercejournal.com",
                    "commercial-news.com",
                    "coopercrier.com",
                    "cordeledispatch.com",
                    "corsicanadailysun.com",
                    "crossville-chronicle.com",
                    "cullmantimes.com",
                    "dailyiowegian.com",
                    "dailyitem.com",
                    "daltondailycitizen.com",
                    "derrynews.com",
                    "duncanbanner.com",
                    "eagletribune.com",
                    "edmondsun.com",
                    "effinghamdailynews.com",
                    "enewscourier.com",
                    "enidnews.com",
                    "farmtalknewspaper.com",
                    "fayettetribune.com",
                    "flasharcade.com",
                    "flashgames247.com",
                    "flyergroup.com",
                    "foxsportsasia.com",
                    "gainesvilleregister.com",
                    "gloucestertimes.com",
                    "goshennews.com",
                    "greensburgdailynews.com",
                    "heraldbanner.com",
                    "heraldbulletin.com",
                    "hgazette.com",
                    "homemagonline.com",
                    "itemonline.com",
                    "jacksonvilleprogress.com",
                    "jerusalemonline.com",
                    "joplinglobe.com",
                    "journal-times.com",
                    "journalexpress.net",
                    "kexp.org",
                    "kokomotribune.com",
                    "lockportjournal.com",
                    "mankatofreepress.com",
                    "mcalesternews.com",
                    "mccrearyrecord.com",
                    "mcleansborotimesleader.com",
                    "meadvilletribune.com",
                    "meridianstar.com",
                    "mineralwellsindex.com",
                    "montgomery-herald.com",
                    "mooreamerican.com",
                    "moultrieobserver.com",
                    "muskogeephoenix.com",
                    "ncnewsonline.com",
                    "newburyportnews.com",
                    "newsaegis.com",
                    "newsandtribune.com",
                    "niagara-gazette.com",
                    "njeffersonnews.com",
                    "normantranscript.com",
                    "opposingviews.com",
                    "orangeleader.com",
                    "oskaloosa.com",
                    "ottumwacourier.com",
                    "outlookmoney.com",
                    "palestineherald.com",
                    "panews.com",
                    "paulsvalleydailydemocrat.com",
                    "pellachronicle.com",
                    "pharostribune.com",
                    "pressrepublican.com",
                    "pryordailytimes.com",
                    "randolphguide.com",
                    "record-eagle.com",
                    "register-herald.com",
                    "register-news.com",
                    "reporter.net",
                    "rockwallheraldbanner.com",
                    "roysecityheraldbanner.com",
                    "rushvillerepublican.com",
                    "salemnews.com",
                    "sentinel-echo.com",
                    "sharonherald.com",
                    "shelbyvilledailyunion.com",
                    "siteslike.com",
                    "standardmedia.co.ke",
                    "starbeacon.com",
                    "stwnewspress.com",
                    "suwanneedemocrat.com",
                    "tahlequahdailypress.com",
                    "theadanews.com",
                    "theawesomer.com",
                    "thedailystar.com",
                    "thelandonline.com",
                    "themoreheadnews.com",
                    "thesnaponline.com",
                    "tiftongazette.com",
                    "times-news.com",
                    "timesenterprise.com",
                    "timessentinel.com",
                    "timeswv.com",
                    "tonawanda-news.com",
                    "tribdem.com",
                    "tribstar.com",
                    "unionrecorder.com",
                    "valdostadailytimes.com",
                    "washtimesherald.com",
                    "waurikademocrat.com",
                    "wcoutlook.com",
                    "weatherforddemocrat.com",
                    "woodwardnews.net",
                    "wrestlinginc.com",
                ]),
                ..Default::default()
            },
        );
    }

    #[test]
    fn href() {
        check_parse_result(
            r#"##a[href$="/vghd.shtml"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"a[href$="/vghd.shtml"]"#.to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"##a[href*=".adk2x.com/"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"a[href*=".adk2x.com/"]"#.to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"##a[href^="//40ceexln7929.com/"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"a[href^="//40ceexln7929.com/"]"#.to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"##a[href*=".trust.zone"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"a[href*=".trust.zone"]"#.to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"tf2maps.net##a[href="http://forums.tf2maps.net/payments.php"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"a[href="http://forums.tf2maps.net/payments.php"]"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec!["tf2maps.net"]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"rarbg.to,rarbg.unblockall.org,rarbgaccess.org,rarbgmirror.com,rarbgmirror.org,rarbgmirror.xyz,rarbgproxy.com,rarbgproxy.org,rarbgunblock.com##a[href][target="_blank"] > button"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"a[href][target="_blank"] > button"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec![
                    "rarbg.to",
                    "rarbg.unblockall.org",
                    "rarbgaccess.org",
                    "rarbgmirror.com",
                    "rarbgmirror.org",
                    "rarbgmirror.xyz",
                    "rarbgproxy.com",
                    "rarbgproxy.org",
                    "rarbgunblock.com",
                ]),
                ..Default::default()
            },
        );
    }

    #[test]
    fn injected_scripts() {
        check_parse_result(
            r#"hentaifr.net,jeu.info,tuxboard.com,xstory-fr.com##+js(goyavelab-defuser.js)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"goyavelab-defuser.js"#.to_string()),
                hostnames: sort_hash_domains(vec![
                    "hentaifr.net",
                    "jeu.info",
                    "tuxboard.com",
                    "xstory-fr.com",
                ]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"haus-garten-test.de,sozialversicherung-kompetent.de##+js(set-constant.js, Object.keys, trueFunc)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"set-constant.js, Object.keys, trueFunc"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec![
                    "haus-garten-test.de",
                    "sozialversicherung-kompetent.de",
                ]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"airliners.de,auszeit.bio,autorevue.at,clever-tanken.de,fanfiktion.de,finya.de,frag-mutti.de,frustfrei-lernen.de,fussballdaten.de,gameswelt.*,liga3-online.de,lz.de,mt.de,psychic.de,rimondo.com,spielen.de,weltfussball.at,weristdeinfreund.de##+js(abort-current-inline-script.js, Number.isNaN)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"abort-current-inline-script.js, Number.isNaN"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec![
                    "airliners.de",
                    "auszeit.bio",
                    "autorevue.at",
                    "clever-tanken.de",
                    "fanfiktion.de",
                    "finya.de",
                    "frag-mutti.de",
                    "frustfrei-lernen.de",
                    "fussballdaten.de",
                    "liga3-online.de",
                    "lz.de",
                    "mt.de",
                    "psychic.de",
                    "rimondo.com",
                    "spielen.de",
                    "weltfussball.at",
                    "weristdeinfreund.de",
                ]),
                entities: sort_hash_domains(vec!["gameswelt"]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"prad.de##+js(abort-on-property-read.js, document.cookie)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"abort-on-property-read.js, document.cookie"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec!["prad.de"]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"computerbild.de##+js(abort-on-property-read.js, Date.prototype.toUTCString)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"abort-on-property-read.js, Date.prototype.toUTCString"#.to_string(),
                ),
                hostnames: sort_hash_domains(vec!["computerbild.de"]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"computerbild.de##+js(setTimeout-defuser.js, ())return)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"setTimeout-defuser.js, ())return"#.to_string()),
                hostnames: sort_hash_domains(vec!["computerbild.de"]),
                script_inject: true,
                ..Default::default()
            },
        );
    }

    #[test]
    fn entities() {
        check_parse_result(
            r#"monova.*##+js(nowebrtc.js)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"nowebrtc.js"#.to_string()),
                entities: sort_hash_domains(vec!["monova"]),
                script_inject: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"monova.*##tr.success.desktop"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"tr.success.desktop"#.to_string()),
                entities: sort_hash_domains(vec!["monova"]),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"monova.*#@#script + [class] > [class]:first-child"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(
                    r#"script + [class] > [class]:first-child"#.to_string(),
                ),
                entities: sort_hash_domains(vec!["monova"]),
                unhide: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"adshort.im,adsrt.*#@#[id*="ScriptRoot"]"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"[id*="ScriptRoot"]"#.to_string()),
                hostnames: sort_hash_domains(vec!["adshort.im"]),
                entities: sort_hash_domains(vec!["adsrt"]),
                unhide: true,
                ..Default::default()
            },
        );
        check_parse_result(
            r#"downloadsource.*##.date:not(dt):style(display: block !important;)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#".date:not(dt)"#.to_string()),
                entities: sort_hash_domains(vec!["downloadsource"]),
                action: Some(CosmeticFilterAction::Style(
                    "display: block !important;".into(),
                )),
                ..Default::default()
            },
        );
    }

    #[test]
    fn styles() {
        check_parse_result(
            r#"chip.de##.video-wrapper > video[style]:style(display:block!important;padding-top:0!important;)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#".video-wrapper > video[style]"#.to_string()),
                hostnames: sort_hash_domains(vec!["chip.de"]),
                action: Some(CosmeticFilterAction::Style(
                    "display:block!important;padding-top:0!important;".into(),
                )),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"allmusic.com##.advertising.medium-rectangle:style(min-height: 1px !important;)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#".advertising.medium-rectangle"#.to_string()),
                hostnames: sort_hash_domains(vec!["allmusic.com"]),
                action: Some(CosmeticFilterAction::Style(
                    "min-height: 1px !important;".into(),
                )),
                ..Default::default()
            },
        );
        #[cfg(feature = "css-validation")]
        check_parse_result(
            r#"quora.com##.signup_wall_prevent_scroll .SiteHeader,.signup_wall_prevent_scroll .LoggedOutFooter,.signup_wall_prevent_scroll .ContentWrapper:style(filter: none !important;)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#".signup_wall_prevent_scroll .SiteHeader, .signup_wall_prevent_scroll .LoggedOutFooter, .signup_wall_prevent_scroll .ContentWrapper"#.to_string()),
                hostnames: sort_hash_domains(vec!["quora.com"]),
                action: Some(CosmeticFilterAction::Style("filter: none !important;".into())),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"imdb.com##body#styleguide-v2:style(background-color: #e3e2dd !important; background-image: none !important;)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"body#styleguide-v2"#.to_string()),
                hostnames: sort_hash_domains(vec!["imdb.com"]),
                action: Some(CosmeticFilterAction::Style(
                    "background-color: #e3e2dd !important; background-image: none !important;"
                        .into(),
                )),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"streamcloud.eu###login > div[style^="width"]:style(display: block !important)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"#login > div[style^="width"]"#.to_string()),
                hostnames: sort_hash_domains(vec!["streamcloud.eu"]),
                action: Some(CosmeticFilterAction::Style(
                    "display: block !important".into(),
                )),
                ..Default::default()
            },
        );
        check_parse_result(
            r#"moonbit.co.in,moondoge.co.in,moonliteco.in##[src^="//coinad.com/ads/"]:style(visibility: collapse !important)"#,
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss(r#"[src^="//coinad.com/ads/"]"#.to_string()),
                hostnames: sort_hash_domains(vec![
                    "moonbit.co.in",
                    "moondoge.co.in",
                    "moonliteco.in",
                ]),
                action: Some(CosmeticFilterAction::Style(
                    "visibility: collapse !important".into(),
                )),
                ..Default::default()
            },
        );
    }

    #[test]
    fn unicode() {
        check_parse_result(
            "###неделя",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("#неделя".to_string()),
                ..Default::default()
            },
        );
        check_parse_result(
            "неlloworlд.com#@##week",
            CosmeticFilterBreakdown {
                selector: SelectorType::PlainCss("#week".to_string()),
                hostnames: sort_hash_domains(vec!["xn--lloworl-5ggb3f.com"]),
                unhide: true,
                ..Default::default()
            },
        );
    }

    /// As of writing, these procedural filters with multiple comma-separated selectors aren't
    /// fully supported by uBO. Here, they are treated as parsing errors.
    #[test]
    #[cfg(feature = "css-validation")]
    fn multi_selector_procedural_filters() {
        assert!(parse_cf("example.com##h1:has-text(Example Domain),p:has-text(More)").is_err());
        assert!(parse_cf("example.com##h1,p:has-text(ill)").is_err());
        assert!(parse_cf("example.com##h1:has-text(om),p").is_err());
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn procedural_operators() {
        /// Check against simple `example.com` domains. Domain parsing is well-handled by other
        /// tests, but procedural filters cannot be generic.
        fn check_procedural(raw: &str, expected_selectors: Vec<CosmeticFilterOperator>) {
            check_parse_result(
                &format!("example.com##{}", raw),
                CosmeticFilterBreakdown {
                    selector: SelectorType::Procedural(expected_selectors),
                    hostnames: sort_hash_domains(vec!["example.com"]),
                    ..Default::default()
                },
            );
        }
        check_procedural(
            ".items:has-text(Sponsored)",
            vec![
                CosmeticFilterOperator::CssSelector(".items".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
            ],
        );
        check_procedural(
            "div.items:has(p):has-text(Sponsored)",
            vec![
                CosmeticFilterOperator::CssSelector("div.items:has(p)".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
            ],
        );
        check_procedural(
            "div.items:has-text(Sponsored):has(p)",
            vec![
                CosmeticFilterOperator::CssSelector("div.items".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
                CosmeticFilterOperator::CssSelector(":has(p)".to_string()),
            ],
        );
        check_procedural(
            ".items:has-text(Sponsored) .container",
            vec![
                CosmeticFilterOperator::CssSelector(".items".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
                CosmeticFilterOperator::CssSelector(" .container".to_string()),
            ],
        );
        check_procedural(
            ".items:has-text(Sponsored) > .container",
            vec![
                CosmeticFilterOperator::CssSelector(".items".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
                CosmeticFilterOperator::CssSelector(" > .container".to_string()),
            ],
        );
        check_procedural(
            ".items:has-text(Sponsored) + .container:has-text(Ad) ~ div",
            vec![
                CosmeticFilterOperator::CssSelector(".items".to_string()),
                CosmeticFilterOperator::HasText("Sponsored".to_string()),
                CosmeticFilterOperator::CssSelector(" + .container".to_string()),
                CosmeticFilterOperator::HasText("Ad".to_string()),
                CosmeticFilterOperator::CssSelector(" ~ div".to_string()),
            ],
        );
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn unsupported() {
        assert!(parse_cf("yandex.*##.serp-item:if(:scope > div.organic div.organic__subtitle:matches-css-after(content: /[Рр]еклама/))").is_err());
        assert!(parse_cf(
            r#"facebook.com,facebookcorewwwi.onion##.ego_column:if(a[href^="/campaign/landing"])"#
        )
        .is_err());
        assert!(parse_cf(r#"readcomiconline.to##^script:has-text(this[atob)"#).is_err());
        assert!(parse_cf("##").is_err());
        assert!(parse_cf("").is_err());

        // `:has` was previously limited to procedural filtering, but is now a native CSS feature.
        assert!(
            parse_cf(r#"thedailywtf.com##.article-body > div:has(a[href*="utm_medium"])"#).is_ok()
        );

        // `:has-text` and `:xpath` are now supported procedural filters
        assert!(parse_cf("twitter.com##article:has-text(/Promoted|Gesponsert|Реклама|Promocionado/):xpath(../..)").is_ok());

        // generic procedural filters are not supported
        assert!(parse_cf("##.t-rec > .t886:has-text(cookies)").is_err());
    }

    #[test]
    fn hidden_generic() {
        let rule = parse_cf("##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.*##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com,~a.test.com##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.*,~a.test.com##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.*,~a.test.*##.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com#@#.selector").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("~test.com##.selector").unwrap();
        assert_eq!(
            CosmeticFilterBreakdown::from(rule.hidden_generic_rule().unwrap()),
            parse_cf("##.selector").unwrap().into(),
        );

        let rule = parse_cf("~test.*##.selector").unwrap();
        assert_eq!(
            CosmeticFilterBreakdown::from(rule.hidden_generic_rule().unwrap()),
            parse_cf("##.selector").unwrap().into(),
        );

        let rule = parse_cf("~test.*,~a.test.*##.selector").unwrap();
        assert_eq!(
            CosmeticFilterBreakdown::from(rule.hidden_generic_rule().unwrap()),
            parse_cf("##.selector").unwrap().into(),
        );

        let rule = parse_cf("test.com##.selector:style(border-radius: 13px)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.*##.selector:style(border-radius: 13px)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("~test.com##.selector:style(border-radius: 13px)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("~test.*##.selector:style(border-radius: 13px)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com#@#.selector:style(border-radius: 13px)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com##+js(nowebrtc.js)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.*##+js(nowebrtc.js)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("~test.com##+js(nowebrtc.js)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("~test.*##+js(nowebrtc.js)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());

        let rule = parse_cf("test.com#@#+js(nowebrtc.js)").unwrap();
        assert!(rule.hidden_generic_rule().is_none());
    }
}

#[cfg(test)]
mod util_tests {
    use super::super::*;
    use crate::utils::fast_hash;

    #[test]
    fn label_hashing() {
        assert_eq!(
            get_hashes_from_labels("foo.bar.baz", 11, 11),
            vec![
                fast_hash("baz"),
                fast_hash("bar.baz"),
                fast_hash("foo.bar.baz")
            ]
        );
        assert_eq!(
            get_hashes_from_labels("foo.bar.baz.com", 15, 8),
            vec![
                fast_hash("baz.com"),
                fast_hash("bar.baz.com"),
                fast_hash("foo.bar.baz.com")
            ]
        );
        assert_eq!(
            get_hashes_from_labels("foo.bar.baz.com", 11, 11),
            vec![
                fast_hash("baz"),
                fast_hash("bar.baz"),
                fast_hash("foo.bar.baz")
            ]
        );
        assert_eq!(
            get_hashes_from_labels("foo.bar.baz.com", 11, 8),
            vec![
                fast_hash("baz"),
                fast_hash("bar.baz"),
                fast_hash("foo.bar.baz")
            ]
        );
    }

    #[test]
    fn without_public_suffix() {
        assert_eq!(get_hostname_without_public_suffix("", ""), None);
        assert_eq!(get_hostname_without_public_suffix("com", ""), None);
        assert_eq!(get_hostname_without_public_suffix("com", "com"), None);
        assert_eq!(
            get_hostname_without_public_suffix("foo.com", "foo.com"),
            Some(("foo", "com"))
        );
        assert_eq!(
            get_hostname_without_public_suffix("foo.bar.com", "bar.com"),
            Some(("foo.bar", "com"))
        );
        assert_eq!(
            get_hostname_without_public_suffix("test.github.io", "test.github.io"),
            Some(("test", "github.io"))
        );
    }
}

#[cfg(test)]
mod matching_tests {
    use super::super::*;
    use crate::utils::bin_lookup;

    trait MatchByStr {
        fn matches(&self, request_entities: &[Hash], request_hostnames: &[Hash]) -> bool;
        fn matches_str(&self, hostname: &str, domain: &str) -> bool;
    }

    impl MatchByStr for CosmeticFilter {
        /// `hostname` and `domain` should be specified as, e.g. "subdomain.domain.com" and
        /// "domain.com", respectively. This function will panic if the specified `domain` is
        /// longer than the specified `hostname`.
        fn matches_str(&self, hostname: &str, domain: &str) -> bool {
            debug_assert!(hostname.len() >= domain.len());

            let request_entities = get_entity_hashes_from_labels(hostname, domain);

            let request_hostnames = get_hostname_hashes_from_labels(hostname, domain);

            self.matches(&request_entities[..], &request_hostnames[..])
        }

        /// Check whether this rule applies to content from the hostname and domain corresponding to
        /// the provided hash lists.
        ///
        /// See the `matches_str` test function for an example of how to convert hostnames and
        /// domains into the appropriate hash lists.
        fn matches(&self, request_entities: &[Hash], request_hostnames: &[Hash]) -> bool {
            let has_hostname_constraint = self.has_hostname_constraint();
            if !has_hostname_constraint {
                return true;
            }
            if request_entities.is_empty()
                && request_hostnames.is_empty()
                && has_hostname_constraint
            {
                return false;
            }

            if let Some(ref filter_not_hostnames) = self.not_hostnames {
                if request_hostnames
                    .iter()
                    .any(|hash| bin_lookup(filter_not_hostnames, *hash))
                {
                    return false;
                }
            }

            if let Some(ref filter_not_entities) = self.not_entities {
                if request_entities
                    .iter()
                    .any(|hash| bin_lookup(filter_not_entities, *hash))
                {
                    return false;
                }
            }

            if self.hostnames.is_some() || self.entities.is_some() {
                if let Some(ref filter_hostnames) = self.hostnames {
                    if request_hostnames
                        .iter()
                        .any(|hash| bin_lookup(filter_hostnames, *hash))
                    {
                        return true;
                    }
                }

                if let Some(ref filter_entities) = self.entities {
                    if request_entities
                        .iter()
                        .any(|hash| bin_lookup(filter_entities, *hash))
                    {
                        return true;
                    }
                }

                return false;
            }

            true
        }
    }

    fn parse_cf(rule: &str) -> Result<CosmeticFilter, CosmeticFilterError> {
        CosmeticFilter::parse(rule, false, Default::default())
    }

    #[test]
    fn generic_filter() {
        let rule = parse_cf("##.selector").unwrap();
        assert!(rule.matches_str("foo.com", "foo.com"));
    }

    #[test]
    fn single_domain() {
        let rule = parse_cf("foo.com##.selector").unwrap();
        assert!(rule.matches_str("foo.com", "foo.com"));
        assert!(!rule.matches_str("bar.com", "bar.com"));
    }

    #[test]
    fn multiple_domains() {
        let rule = parse_cf("foo.com,test.com##.selector").unwrap();
        assert!(rule.matches_str("foo.com", "foo.com"));
        assert!(rule.matches_str("test.com", "test.com"));
        assert!(!rule.matches_str("bar.com", "bar.com"));
    }

    #[test]
    fn subdomain() {
        let rule = parse_cf("foo.com,test.com##.selector").unwrap();
        assert!(rule.matches_str("sub.foo.com", "foo.com"));
        assert!(rule.matches_str("sub.test.com", "test.com"));

        let rule = parse_cf("foo.com,sub.test.com##.selector").unwrap();
        assert!(rule.matches_str("sub.test.com", "test.com"));
        assert!(!rule.matches_str("test.com", "test.com"));
        assert!(!rule.matches_str("com", "com"));
    }

    #[test]
    fn entity() {
        let rule = parse_cf("foo.com,sub.test.*##.selector").unwrap();
        assert!(rule.matches_str("foo.com", "foo.com"));
        assert!(rule.matches_str("bar.foo.com", "foo.com"));
        assert!(rule.matches_str("sub.test.com", "test.com"));
        assert!(rule.matches_str("sub.test.fr", "test.fr"));
        assert!(!rule.matches_str("sub.test.evil.biz", "evil.biz"));

        let rule = parse_cf("foo.*##.selector").unwrap();
        assert!(rule.matches_str("foo.co.uk", "foo.co.uk"));
        assert!(rule.matches_str("bar.foo.co.uk", "foo.co.uk"));
        assert!(rule.matches_str("baz.bar.foo.co.uk", "foo.co.uk"));
        assert!(!rule.matches_str("foo.evil.biz", "evil.biz"));
    }

    #[test]
    fn nonmatching() {
        let rule = parse_cf("foo.*##.selector").unwrap();
        assert!(!rule.matches_str("foo.bar.com", "bar.com"));
        assert!(!rule.matches_str("bar-foo.com", "bar-foo.com"));
    }

    #[test]
    fn entity_negations() {
        let rule = parse_cf("~foo.*##.selector").unwrap();
        assert!(!rule.matches_str("foo.com", "foo.com"));
        assert!(rule.matches_str("foo.evil.biz", "evil.biz"));

        let rule = parse_cf("~foo.*,~bar.*##.selector").unwrap();
        assert!(rule.matches_str("baz.com", "baz.com"));
        assert!(!rule.matches_str("foo.com", "foo.com"));
        assert!(!rule.matches_str("sub.foo.com", "foo.com"));
        assert!(!rule.matches_str("bar.com", "bar.com"));
        assert!(!rule.matches_str("sub.bar.com", "bar.com"));
    }

    #[test]
    fn hostname_negations() {
        let rule = parse_cf("~foo.com##.selector").unwrap();
        assert!(!rule.matches_str("foo.com", "foo.com"));
        assert!(!rule.matches_str("bar.foo.com", "foo.com"));
        assert!(rule.matches_str("foo.com.bar", "com.bar"));
        assert!(rule.matches_str("foo.co.uk", "foo.co.uk"));

        let rule = parse_cf("~foo.com,~foo.de,~bar.com##.selector").unwrap();
        assert!(!rule.matches_str("foo.com", "foo.com"));
        assert!(!rule.matches_str("sub.foo.com", "foo.com"));
        assert!(!rule.matches_str("foo.de", "foo.de"));
        assert!(!rule.matches_str("sub.foo.de", "foo.de"));
        assert!(!rule.matches_str("bar.com", "bar.com"));
        assert!(!rule.matches_str("sub.bar.com", "bar.com"));
        assert!(rule.matches_str("bar.de", "bar.de"));
        assert!(rule.matches_str("sub.bar.de", "bar.de"));
    }

    #[test]
    fn entity_with_suffix_exception() {
        let rule = parse_cf("foo.*,~foo.com##.selector").unwrap();
        assert!(!rule.matches_str("foo.com", "foo.com"));
        assert!(!rule.matches_str("sub.foo.com", "foo.com"));
        assert!(rule.matches_str("foo.de", "foo.de"));
        assert!(rule.matches_str("sub.foo.de", "foo.de"));
    }

    #[test]
    fn entity_with_subdomain_exception() {
        let rule = parse_cf("foo.*,~sub.foo.*##.selector").unwrap();
        assert!(rule.matches_str("foo.com", "foo.com"));
        assert!(rule.matches_str("foo.de", "foo.de"));
        assert!(!rule.matches_str("sub.foo.com", "foo.com"));
        assert!(!rule.matches_str("bar.com", "bar.com"));
        assert!(rule.matches_str("sub2.foo.com", "foo.com"));
    }

    #[test]
    fn no_domain_provided() {
        let rule = parse_cf("foo.*##.selector").unwrap();
        assert!(!rule.matches_str("foo.com", ""));
    }

    #[test]
    fn no_hostname_provided() {
        let rule = parse_cf("domain.com##.selector").unwrap();
        assert!(!rule.matches_str("", ""));
        let rule = parse_cf("domain.*##.selector").unwrap();
        assert!(!rule.matches_str("", ""));
        let rule = parse_cf("~domain.*##.selector").unwrap();
        assert!(!rule.matches_str("", ""));
        let rule = parse_cf("~domain.com##.selector").unwrap();
        assert!(!rule.matches_str("", ""));
    }

    #[test]
    fn respects_etld() {
        let rule = parse_cf("github.io##.selector").unwrap();
        assert!(rule.matches_str("test.github.io", "github.io"));
    }

    #[test]
    fn multiple_selectors() {
        assert!(
            parse_cf("youtube.com##.masthead-ad-control,.ad-div,.pyv-afc-ads-container").is_ok()
        );
        assert!(parse_cf("m.economictimes.com###appBanner,#stickyBanner").is_ok());
        assert!(parse_cf("googledrivelinks.com###wpsafe-generate, #wpsafe-link:style(display: block !important;)").is_ok());
    }

    #[test]
    fn actions() {
        assert!(parse_cf("example.com###adBanner:style(background: transparent)").is_ok());
        assert!(parse_cf("example.com###adBanner:remove()").is_ok());
        assert!(parse_cf("example.com###adBanner:remove-attr(style)").is_ok());
        assert!(parse_cf("example.com###adBanner:remove-class(src)").is_ok());
    }

    #[test]
    fn zero_width_space() {
        assert!(parse_cf(r#"​##a[href^="https://www.g2fame.com/"] > img"#).is_err());
    }

    #[test]
    fn adg_regex() {
        assert!(parse_cf(r"/^dizipal\d+\.com$/##.web").is_err());
        // Filter is still salvageable if at least one location is supported
        assert!(parse_cf(r"/^dizipal\d+\.com,test.net$/##.web").is_ok());
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn abp_has_conversion() {
        let rule =
            parse_cf("imgur.com#?#div.Gallery-Sidebar-PostContainer:-abp-has(div.promoted-hover)")
                .unwrap();
        assert_eq!(
            rule.plain_css_selector(),
            Some("div.Gallery-Sidebar-PostContainer:has(div.promoted-hover)")
        );
        let rule =
            parse_cf(r##"webtools.fineaty.com#?#div[class*=" hidden-"]:-abp-has(.adsbygoogle)"##)
                .unwrap();
        assert_eq!(
            rule.plain_css_selector(),
            Some(r#"div[class*=" hidden-"]:has(.adsbygoogle)"#)
        );
        let rule = parse_cf(r##"facebook.com,facebookcorewwwi.onion#?#._6y8t:-abp-has(a[href="/ads/about/?entry_product=ad_preferences"])"##).unwrap();
        assert_eq!(
            rule.plain_css_selector(),
            Some(r#"._6y8t:has(a[href="/ads/about/?entry_product=ad_preferences"])"#)
        );
        let rule =
            parse_cf(r##"mtgarena.pro#?##root > div > div:-abp-has(> .vm-placement)"##).unwrap();
        assert_eq!(
            rule.plain_css_selector(),
            Some(r#"#root > div > div:has(> .vm-placement)"#)
        );
        // Error without `#?#`:
        assert!(
            parse_cf(r##"mtgarena.pro###root > div > div:-abp-has(> .vm-placement)"##).is_err()
        );
    }
}

#[cfg(test)]
#[cfg(feature = "css-validation")]
mod css_validation_tests {
    use super::super::*;

    #[test]
    fn bad_selector_inputs() {
        assert!(validate_css_selector(r#"rm -rf ./*"#, false).is_err());
        assert!(validate_css_selector(
            r#"javascript:alert("All pseudo-classes are valid")"#,
            false
        )
        .is_ok());
        assert!(validate_css_selector(
            r#"javascript:alert("But opening comments are still forbidden" /*)"#,
            false
        )
        .is_err());
        assert!(validate_css_selector(r#"This is not a CSS selector."#, false).is_err());
        assert!(validate_css_selector(r#"./malware.sh"#, false).is_err());
        assert!(validate_css_selector(r#"https://safesite.ru"#, false).is_err());
        assert!(validate_css_selector(
            r#"(function(){var e=60;return String.fromCharCode(e.charCodeAt(0))})();"#,
            false
        )
        .is_err());
        assert!(validate_css_selector(r#"#!/usr/bin/sh"#, false).is_err());
        assert!(validate_css_selector(r#"input,input/*"#, false).is_err());
        // Accept a closing comment within a string. It should still be impossible to create an
        // opening comment to match it.
        assert!(validate_css_selector(
            r#"input[x="*/{}*{background:url(https://hackvertor.co.uk/images/logo.gif)}"]"#,
            false
        )
        .is_ok());
    }

    #[test]
    fn escaped_quote_in_tag_name() {
        assert_eq!(
            validate_css_selector(r#"head\""#, false),
            Ok(vec![CosmeticFilterOperator::CssSelector(
                r#"head\""#.to_string()
            )])
        );
    }
}
