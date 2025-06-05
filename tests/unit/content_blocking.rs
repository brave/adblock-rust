#[cfg(test)]
mod ab2cb_tests {
    use super::super::*;

    fn test_from_abp(abp_rule: &str, cb: &str) {
        let filter = crate::lists::parse_filter(abp_rule, true, Default::default())
            .expect("Rule under test could not be parsed");
        assert_eq!(
            CbRuleEquivalent::try_from(filter)
                .unwrap()
                .into_iter()
                .collect::<Vec<_>>(),
            serde_json::from_str::<Vec<CbRule>>(cb)
                .expect("content blocking rule under test could not be deserialized")
        );
    }

    #[test]
    fn ad_tests() {
        test_from_abp(
            "&ad_box_",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "&ad_box_"
            }
        }]"####,
        );
        test_from_abp(
            "&ad_channel=",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "&ad_channel="
            }
        }]"####,
        );
        test_from_abp(
            "+advertorial.",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "\\+advertorial\\."
            }
        }]"####,
        );
        test_from_abp(
            "&prvtof=*&poru=",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "&prvtof=.*&poru="
            }
        }]"####,
        );
        test_from_abp(
            "-ad-180x150px.",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "-ad-180x150px\\."
            }
        }]"####,
        );
        test_from_abp(
            "://findnsave.*.*/api/groupon.json?",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "://findnsave\\..*\\..*/api/groupon\\.json\\?"
            }
        }]"####,
        );
        test_from_abp(
            "|https://$script,third-party,domain=tamilrockers.ws",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "if-domain": ["*tamilrockers.ws"],
                "load-type": ["third-party"],
                "resource-type": ["script"],
                "url-filter": "^https://"
            }
        }]"####,
        );
        test_from_abp("||com/banners/$image,object,subdocument,domain=~pingdom.com|~thetvdb.com|~tooltrucks.com", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?com/banners/",
                "unless-domain": [
                    "*pingdom.com",
                    "*thetvdb.com",
                    "*tooltrucks.com"
                ],
                "resource-type": [
                    "image"
                ]
            }
        }, {
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?com/banners/",
                "unless-domain": [
                    "*pingdom.com",
                    "*thetvdb.com",
                    "*tooltrucks.com"
                ],
                "resource-type": [
                    "document"
                ],
                "load-type": [
                    "third-party"
                ]
            },
            "action": {
                "type": "block"
            }
        }]"####);
        test_from_abp(
            "$image,third-party,xmlhttprequest,domain=rd.com",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^https?://",
                "if-domain": [
                    "*rd.com"
                ],
                "resource-type": [
                    "image",
                    "raw"
                ],
                "load-type": [
                    "third-party"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "|https://r.i.ua^",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^https://r\\.i\\.ua"
            }
        }]"####,
        );
        test_from_abp(
            "|ws://$domain=4shared.com",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^wss?://",
                "if-domain": [
                    "*4shared.com"
                ]
            }
        }]"####,
        );
    }

    #[test]
    fn element_hiding_tests() {
        test_from_abp(
            "###A9AdsMiddleBoxTop",
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "#A9AdsMiddleBoxTop"
            },
            "trigger": {
                "url-filter": ".*"
            }
        }]"####,
        );
        test_from_abp(
            "thedailygreen.com#@##AD_banner",
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "#AD_banner"
            },
            "trigger": {
                "url-filter": ".*",
                "unless-domain": [
                    "thedailygreen.com"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "sprouts.com,tbns.com.au#@##AdImage",
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "#AdImage"
            },
            "trigger": {
                "url-filter": ".*",
                "unless-domain": [
                    "sprouts.com",
                    "tbns.com.au"
                ]
            }
        }]"####,
        );
        test_from_abp(
            r#"santander.co.uk#@#a[href^="http://ad-emea.doubleclick.net/"]"#,
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "a[href^=\"http://ad-emea.doubleclick.net/\"]"
            },
            "trigger": {
                "url-filter": ".*",
                "unless-domain": [
                    "santander.co.uk"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "search.safefinder.com,search.snapdo.com###ABottomD",
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "#ABottomD"
            },
            "trigger": {
                "url-filter": ".*",
                "if-domain": [
                    "search.safefinder.com",
                    "search.snapdo.com"
                ]
            }
        }]"####,
        );
        test_from_abp(
            r#"tweakguides.com###adbar > br + p[style="text-align: center"] + p[style="text-align: center"]"#,
            r####"[{
            "action": {
                "type": "css-display-none",
                "selector": "#adbar > br + p[style=\"text-align: center\"] + p[style=\"text-align: center\"]"
            },
            "trigger": {
                "url-filter": ".*",
                "if-domain": [
                    "tweakguides.com"
                ]
            }
        }]"####,
        );
    }

    /* TODO - `$popup` is currently unsupported by NetworkFilter
    #[test]
    fn popup_tests() {
        test_from_abp("||admngronline.com^$popup,third-party", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^https?://admngronline\\.com(?:[\\x00-\\x24\\x26-\\x2C\\x2F\\x3A-\\x40\\x5B-\\x5E\\x60\\x7B-\\x7F]|$)",
                "load-type": [
                    "third-party"
                ],
                "resource-type": [
                    "popup"
                ]
            }
        }]"####);
        test_from_abp("||bet365.com^*affiliate=$popup", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^https?://bet365\\.com(?:[\\x00-\\x24\\x26-\\x2C\\x2F\\x3A-\\x40\\x5B-\\x5E\\x60\\x7B-\\x7F]|$).*affiliate=",
                "resource-type": [
                    "popup"
                ]
            }
        }]"####);
    }
    */

    #[test]
    fn third_party() {
        test_from_abp(
            "||007-gateway.com^$third-party",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?007-gateway\\.com",
                "load-type": [
                    "third-party"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "||allestörungen.at^$third-party",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?xn--allestrungen-9ib\\.at",
                "load-type": [
                    "third-party"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "||anet*.tradedoubler.com^$third-party",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?anet.*\\.tradedoubler\\.com",
                "load-type": [
                    "third-party"
                ]
            }
        }]"####,
        );
        test_from_abp("||doubleclick.net^$third-party,domain=3news.co.nz|92q.com|abc-7.com|addictinggames.com|allbusiness.com|allthingsd.com|bizjournals.com|bloomberg.com|bnn.ca|boom92houston.com|boom945.com|boomphilly.com|break.com|cbc.ca|cbs19.tv|cbs3springfield.com|cbsatlanta.com|cbslocal.com|complex.com|dailymail.co.uk|darkhorizons.com|doubleviking.com|euronews.com|extratv.com|fandango.com|fox19.com|fox5vegas.com|gorillanation.com|hawaiinewsnow.com|hellobeautiful.com|hiphopnc.com|hot1041stl.com|hothiphopdetroit.com|hotspotatl.com|hulu.com|imdb.com|indiatimes.com|indyhiphop.com|ipowerrichmond.com|joblo.com|kcra.com|kctv5.com|ketv.com|koat.com|koco.com|kolotv.com|kpho.com|kptv.com|ksat.com|ksbw.com|ksfy.com|ksl.com|kypost.com|kysdc.com|live5news.com|livestation.com|livestream.com|metro.us|metronews.ca|miamiherald.com|my9nj.com|myboom1029.com|mycolumbusmagic.com|mycolumbuspower.com|myfoxdetroit.com|myfoxorlando.com|myfoxphilly.com|myfoxphoenix.com|myfoxtampabay.com|nbcrightnow.com|neatorama.com|necn.com|neopets.com|news.com.au|news4jax.com|newsone.com|nintendoeverything.com|oldschoolcincy.com|own3d.tv|pagesuite-professional.co.uk|pandora.com|player.theplatform.com|ps3news.com|radio.com|radionowindy.com|rottentomatoes.com|sbsun.com|shacknews.com|sk-gaming.com|ted.com|thebeatdfw.com|theboxhouston.com|theglobeandmail.com|timesnow.tv|tv2.no|twitch.tv|universalsports.com|ustream.tv|wapt.com|washingtonpost.com|wate.com|wbaltv.com|wcvb.com|wdrb.com|wdsu.com|wflx.com|wfmz.com|wfsb.com|wgal.com|whdh.com|wired.com|wisn.com|wiznation.com|wlky.com|wlns.com|wlwt.com|wmur.com|wnem.com|wowt.com|wral.com|wsj.com|wsmv.com|wsvn.com|wtae.com|wthr.com|wxii12.com|wyff4.com|yahoo.com|youtube.com|zhiphopcleveland.com", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?doubleclick\\.net",
                "load-type": [
                    "third-party"
                ],
                "if-domain": [
                    "*3news.co.nz",
                    "*92q.com",
                    "*abc-7.com",
                    "*addictinggames.com",
                    "*allbusiness.com",
                    "*allthingsd.com",
                    "*bizjournals.com",
                    "*bloomberg.com",
                    "*bnn.ca",
                    "*boom92houston.com",
                    "*boom945.com",
                    "*boomphilly.com",
                    "*break.com",
                    "*cbc.ca",
                    "*cbs19.tv",
                    "*cbs3springfield.com",
                    "*cbsatlanta.com",
                    "*cbslocal.com",
                    "*complex.com",
                    "*dailymail.co.uk",
                    "*darkhorizons.com",
                    "*doubleviking.com",
                    "*euronews.com",
                    "*extratv.com",
                    "*fandango.com",
                    "*fox19.com",
                    "*fox5vegas.com",
                    "*gorillanation.com",
                    "*hawaiinewsnow.com",
                    "*hellobeautiful.com",
                    "*hiphopnc.com",
                    "*hot1041stl.com",
                    "*hothiphopdetroit.com",
                    "*hotspotatl.com",
                    "*hulu.com",
                    "*imdb.com",
                    "*indiatimes.com",
                    "*indyhiphop.com",
                    "*ipowerrichmond.com",
                    "*joblo.com",
                    "*kcra.com",
                    "*kctv5.com",
                    "*ketv.com",
                    "*koat.com",
                    "*koco.com",
                    "*kolotv.com",
                    "*kpho.com",
                    "*kptv.com",
                    "*ksat.com",
                    "*ksbw.com",
                    "*ksfy.com",
                    "*ksl.com",
                    "*kypost.com",
                    "*kysdc.com",
                    "*live5news.com",
                    "*livestation.com",
                    "*livestream.com",
                    "*metro.us",
                    "*metronews.ca",
                    "*miamiherald.com",
                    "*my9nj.com",
                    "*myboom1029.com",
                    "*mycolumbusmagic.com",
                    "*mycolumbuspower.com",
                    "*myfoxdetroit.com",
                    "*myfoxorlando.com",
                    "*myfoxphilly.com",
                    "*myfoxphoenix.com",
                    "*myfoxtampabay.com",
                    "*nbcrightnow.com",
                    "*neatorama.com",
                    "*necn.com",
                    "*neopets.com",
                    "*news.com.au",
                    "*news4jax.com",
                    "*newsone.com",
                    "*nintendoeverything.com",
                    "*oldschoolcincy.com",
                    "*own3d.tv",
                    "*pagesuite-professional.co.uk",
                    "*pandora.com",
                    "*player.theplatform.com",
                    "*ps3news.com",
                    "*radio.com",
                    "*radionowindy.com",
                    "*rottentomatoes.com",
                    "*sbsun.com",
                    "*shacknews.com",
                    "*sk-gaming.com",
                    "*ted.com",
                    "*thebeatdfw.com",
                    "*theboxhouston.com",
                    "*theglobeandmail.com",
                    "*timesnow.tv",
                    "*tv2.no",
                    "*twitch.tv",
                    "*universalsports.com",
                    "*ustream.tv",
                    "*wapt.com",
                    "*washingtonpost.com",
                    "*wate.com",
                    "*wbaltv.com",
                    "*wcvb.com",
                    "*wdrb.com",
                    "*wdsu.com",
                    "*wflx.com",
                    "*wfmz.com",
                    "*wfsb.com",
                    "*wgal.com",
                    "*whdh.com",
                    "*wired.com",
                    "*wisn.com",
                    "*wiznation.com",
                    "*wlky.com",
                    "*wlns.com",
                    "*wlwt.com",
                    "*wmur.com",
                    "*wnem.com",
                    "*wowt.com",
                    "*wral.com",
                    "*wsj.com",
                    "*wsmv.com",
                    "*wsvn.com",
                    "*wtae.com",
                    "*wthr.com",
                    "*wxii12.com",
                    "*wyff4.com",
                    "*yahoo.com",
                    "*youtube.com",
                    "*zhiphopcleveland.com"
                ]
            }
        }]"####);
        test_from_abp("||dt00.net^$third-party,domain=~marketgid.com|~marketgid.ru|~marketgid.ua|~mgid.com|~thechive.com", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?dt00\\.net",
                "load-type": [
                    "third-party"
                ],
                "unless-domain": [
                    "*marketgid.com",
                    "*marketgid.ru",
                    "*marketgid.ua",
                    "*mgid.com",
                    "*thechive.com"
                ]
            }
        }]"####);
        test_from_abp("||amazonaws.com/newscloud-production/*/backgrounds/$domain=crescent-news.com|daily-jeff.com|recordpub.com|state-journal.com|the-daily-record.com|the-review.com|times-gazette.com", r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?amazonaws\\.com/newscloud-production/.*/backgrounds/",
                "if-domain": [
                    "*crescent-news.com",
                    "*daily-jeff.com",
                    "*recordpub.com",
                    "*state-journal.com",
                    "*the-daily-record.com",
                    "*the-review.com",
                    "*times-gazette.com"
                ]
            }
        }]"####);
        test_from_abp(
            "||d1noellhv8fksc.cloudfront.net^",
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?d1noellhv8fksc\\.cloudfront\\.net"
            }
        }]"####,
        );
    }

    #[test]
    fn whitelist() {
        test_from_abp(
            "@@||google.com/recaptcha/$domain=mediafire.com",
            r####"[{
            "action": {
                "type": "ignore-previous-rules"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?google\\.com/recaptcha/",
                "if-domain": [
                    "*mediafire.com"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "@@||ad4.liverail.com/?compressed|$domain=majorleaguegaming.com|pbs.org|wikihow.com",
            r####"[{
            "action": {
                "type": "ignore-previous-rules"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?ad4\\.liverail\\.com/\\?compressed$",
                "if-domain": [
                    "*majorleaguegaming.com",
                    "*pbs.org",
                    "*wikihow.com"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "@@||googletagservices.com/tag/js/gpt.js$domain=allestoringen.nl|allestörungen.at",
            r####"[{
            "action": {
                "type": "ignore-previous-rules"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?googletagservices\\.com/tag/js/gpt\\.js",
                "if-domain": [
                    "*allestoringen.nl",
                    "*xn--allestrungen-9ib.at"
                ]
            }
        }]"####,
        );
        test_from_abp(
            "@@||advertising.autotrader.co.uk^$~third-party",
            r####"[{
            "action": {
                "type": "ignore-previous-rules"
            },
            "trigger": {
                "load-type": [
                    "first-party"
                ],
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?advertising\\.autotrader\\.co\\.uk"
            }
        }]"####,
        );
        test_from_abp(
            "@@||advertising.racingpost.com^$image,script,stylesheet,~third-party,xmlhttprequest",
            r####"[{
            "action": {
                "type": "ignore-previous-rules"
            },
            "trigger": {
                "load-type": [
                    "first-party"
                ],
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?advertising\\.racingpost\\.com",
                "resource-type": [
                    "image",
                    "style-sheet",
                    "script",
                    "raw"
                ]
            }
        }]"####,
        );
    }

    #[test]
    fn test_ignore_previous_fp_documents() {
        assert_eq!(
            vec![ignore_previous_fp_documents()],
            serde_json::from_str::<Vec<CbRule>>(
                r####"[{
            "trigger":{
                "url-filter":".*",
                "resource-type":["document"],
                "load-type":["first-party"]
            },
            "action":{"type":"ignore-previous-rules"}
        }]"####
            )
            .expect("content blocking rule under test could not be deserialized")
        );
    }

    #[test]
    fn escape_literal_backslashes() {
        test_from_abp(
            r#"||gamer.no/?module=Tumedia\DFProxy\Modules^"#,
            r####"[{
            "action": {
                "type": "block"
            },
            "trigger": {
                "url-filter": "^[^:]+:(//)?([^/]+\\.)?gamer\\.no/\\?module=tumedia\\\\dfproxy\\\\modules"
            }
        }]"####,
        );
    }
}

#[cfg(test)]
mod filterset_tests {
    use crate::lists::{FilterSet, ParseOptions, RuleTypes};

    const FILTER_LIST: &[&str] = &[
        "||example.com^$script",
        "||test.net^$image,third-party",
        "/trackme.js^$script",
        "example.com##.ad-banner",
        "##.ad-640x480",
        "##p.sponsored",
    ];

    #[test]
    fn convert_all_rules() -> Result<(), ()> {
        let mut set = FilterSet::new(true);
        set.add_filters(FILTER_LIST, Default::default());

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules, FILTER_LIST);

        // All 6 rules plus `ignore_previous_fp_documents()`
        assert_eq!(cb_rules.len(), 7);

        Ok(())
    }

    #[test]
    fn convert_network_only() -> Result<(), ()> {
        let parse_opts = ParseOptions {
            rule_types: RuleTypes::NetworkOnly,
            ..Default::default()
        };

        let mut set = FilterSet::new(true);
        set.add_filters(FILTER_LIST, parse_opts);

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules, &FILTER_LIST[0..3]);

        // 3 network rules plus `ignore_previous_fp_documents()`
        assert_eq!(cb_rules.len(), 4);

        Ok(())
    }

    #[test]
    fn convert_cosmetic_only() -> Result<(), ()> {
        let parse_opts = ParseOptions {
            rule_types: RuleTypes::CosmeticOnly,
            ..Default::default()
        };

        let mut set = FilterSet::new(true);
        set.add_filters(FILTER_LIST, parse_opts);

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules, &FILTER_LIST[3..6]);

        // 3 cosmetic rules only
        assert_eq!(cb_rules.len(), 3);

        Ok(())
    }

    #[test]
    fn ignore_unsupported_rules() -> Result<(), ()> {
        let mut set = FilterSet::new(true);
        set.add_filters(FILTER_LIST, Default::default());
        set.add_filters(
            [
                // unicode characters
                "||rgmechanics.info/uploads/660х90_",
                "||insaattrendy.com/Upload/bükerbanner*.jpg",
                // from domain
                "/siropu/am/core.min.js$script,important,from=~audi-sport.net|~hifiwigwam.com",
                // leading zero-width space
                r#"​##a[href^="https://www.g2fame.com/"] > img"#,
            ],
            Default::default(),
        );

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules, FILTER_LIST);

        // All 6 rules plus `ignore_previous_fp_documents()`
        assert_eq!(cb_rules.len(), 7);

        Ok(())
    }

    #[test]
    fn punycode_if_domains() -> Result<(), ()> {
        let list = [
            "smskaraborg.se,örnsköldsviksgymnasium.se,mojligheternashusab.se##.env-modal-dialog__backdrop",
        ];
        let mut set = FilterSet::new(true);
        set.add_filters(list, Default::default());

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules, list);

        assert_eq!(cb_rules.len(), 1);
        assert!(cb_rules[0].trigger.if_domain.is_some());
        assert_eq!(
            cb_rules[0].trigger.if_domain.as_ref().unwrap(),
            &[
                "smskaraborg.se",
                "xn--rnskldsviksgymnasium-29be.se",
                "mojligheternashusab.se"
            ]
        );

        Ok(())
    }

    #[test]
    fn convert_cosmetic_filter_locations() -> Result<(), ()> {
        let list = [
            r"/^dizipal\d+\.com$/##.web",
            r"/^example\d+\.com$/,test.net,b.*##.ad",
        ];
        let mut set = FilterSet::new(true);
        set.add_filters(list, Default::default());

        let (cb_rules, used_rules) = set.into_content_blocking()?;
        assert_eq!(used_rules.len(), 1);
        assert_eq!(cb_rules.len(), 1);
        assert!(cb_rules[0].trigger.if_domain.is_some());
        assert_eq!(
            cb_rules[0].trigger.if_domain.as_ref().unwrap(),
            &["test.net"]
        );

        Ok(())
    }
}
