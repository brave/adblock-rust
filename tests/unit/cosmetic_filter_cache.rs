#[cfg(test)]
mod key_from_selector_tests {
    use crate::cosmetic_filter_utils::key_from_selector;

    #[test]
    fn no_escapes() {
        assert_eq!(key_from_selector(r#"#selector"#).unwrap(), "#selector");
        assert_eq!(
            key_from_selector(r#"#ad-box[href="https://popads.net"]"#).unwrap(),
            "#ad-box"
        );
        assert_eq!(key_from_selector(r#".p"#).unwrap(), ".p");
        assert_eq!(key_from_selector(r#".ad #ad.adblockblock"#).unwrap(), ".ad");
        assert_eq!(
            key_from_selector(r#"#container.contained"#).unwrap(),
            "#container"
        );
    }

    #[test]
    fn escaped_characters() {
        assert_eq!(
            key_from_selector(r"#Meebo\:AdElement\.Root").unwrap(),
            "#Meebo:AdElement.Root"
        );
        assert_eq!(
            key_from_selector(r"#\ Banner\ Ad\ -\ 590\ x\ 90").unwrap(),
            "# Banner Ad - 590 x 90"
        );
        assert_eq!(key_from_selector(r"#\ rek").unwrap(), "# rek");
        assert_eq!(
            key_from_selector(r#"#\:rr .nH[role="main"] .mq:first-child"#).unwrap(),
            "#:rr"
        );
        assert_eq!(
            key_from_selector(r#"#adspot-300x600\,300x250-pos-1"#).unwrap(),
            "#adspot-300x600,300x250-pos-1"
        );
        assert_eq!(
            key_from_selector(r#"#adv_\'146\'"#).unwrap(),
            "#adv_\'146\'"
        );
        assert_eq!(
            key_from_selector(r#"#oas-mpu-left\<\/div\>"#).unwrap(),
            "#oas-mpu-left</div>"
        );
        assert_eq!(
            key_from_selector(r#".Trsp\(op\).Trsdu\(3s\)"#).unwrap(),
            ".Trsp(op)"
        );
    }

    #[test]
    fn escape_codes() {
        assert_eq!(
            key_from_selector(r#"#\5f _mom_ad_12"#).unwrap(),
            "#__mom_ad_12"
        );
        assert_eq!(
            key_from_selector(r#"#\5f _nq__hh[style="display:block!important"]"#).unwrap(),
            "#__nq__hh"
        );
        assert_eq!(
            key_from_selector(r#"#\31 000-014-ros"#).unwrap(),
            "#1000-014-ros"
        );
        assert_eq!(key_from_selector(r#"#\33 00X250ad"#).unwrap(), "#300X250ad");
        assert_eq!(key_from_selector(r#"#\5f _fixme"#).unwrap(), "#__fixme");
        assert_eq!(key_from_selector(r#"#\37 28ad"#).unwrap(), "#728ad");
    }

    #[test]
    fn bad_escapes() {
        assert!(key_from_selector(r#"#\5ffffffffff overflows"#).is_none());
        assert!(key_from_selector(r#"#\5fffffff is_too_large"#).is_none());
    }
}

#[cfg(test)]
mod cosmetic_cache_tests {
    use super::super::*;
    use crate::resources::Resource;
    use base64::{engine::Engine as _, prelude::BASE64_STANDARD};

    fn cache_from_rules(rules: Vec<&str>) -> CosmeticFilterCache {
        let parsed_rules = rules
            .iter()
            .map(|r| CosmeticFilter::parse(r, false, Default::default()).unwrap())
            .collect::<Vec<_>>();

        CosmeticFilterCache::from_rules(parsed_rules)
    }

    #[test]
    fn exceptions() {
        let cfcache = cache_from_rules(vec!["~example.com##.item", "sub.example.com#@#.item2"]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "example.com", false);
        expected.exceptions.insert(".item".into());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        expected.exceptions.insert(".item2".into());
        assert_eq!(out, expected);
    }

    #[test]
    fn exceptions2() {
        let cfcache = cache_from_rules(vec!["example.com,~sub.example.com##.item"]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "example.com", false);
        expected.hide_selectors.insert(".item".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        expected.exceptions.insert(".item".into());
        assert_eq!(out, expected);
    }

    #[test]
    fn style_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.element:style(background: #fff)",
            "sub.test.example.com#@#.element:style(background: #fff)",
            "a1.sub.example.com##.element",
            "a2.sub.example.com##.element:style(background: #000)",
            "a3.example.com##.element:style(background: #000)",
        ]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.test.example.com", false);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a1.sub.example.com", false);
        expected.hide_selectors.insert(".element".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.example.com", false);
        expected.hide_selectors.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter::from_css(
                ".element".to_string(),
                "background: #fff".to_string(),
            ))
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a2.sub.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter::from_css(
                ".element".to_string(),
                "background: #000".to_string(),
            ))
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a3.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter::from_css(
                ".element".to_string(),
                "background: #000".to_string(),
            ))
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter::from_css(
                ".element".to_string(),
                "background: #fff".to_string(),
            ))
            .unwrap(),
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn script_exceptions() {
        use crate::resources::{MimeType, ResourceType};

        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##+js(set-constant.js, atob, trueFunc)",
            "sub.test.example.com#@#+js(set-constant.js, atob, trueFunc)",
            "cosmetic.net##+js(nowebrtc.js)",
            "g.cosmetic.net##+js(window.open-defuser.js)",
            "c.g.cosmetic.net#@#+js(nowebrtc.js)",
            "d.g.cosmetic.net#@#+js()",
        ]);
        let resources = ResourceStorage::from_resources([
            Resource {
                name: "set-constant.js".into(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: BASE64_STANDARD.encode("set-constant.js, {{1}}, {{2}}"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource::simple(
                "nowebrtc.js",
                MimeType::ApplicationJavascript,
                "nowebrtc.js",
            ),
            Resource::simple(
                "window.open-defuser.js",
                MimeType::ApplicationJavascript,
                "window.open-defuser.js",
            ),
        ]);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.test.example.com", false);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.example.com", false);
        expected.injected_script =
            "try {\nset-constant.js, atob, trueFunc\n} catch ( e ) { }\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "cosmetic.net", false);
        expected.injected_script = "try {\nnowebrtc.js\n} catch ( e ) { }\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "g.cosmetic.net", false);
        expected.injected_script = "try {\nnowebrtc.js\n} catch ( e ) { }\ntry {\nwindow.open-defuser.js\n} catch ( e ) { }\n".to_owned();
        // order is non-deterministic
        if out != expected {
            expected.injected_script = "try {\nwindow.open-defuser.js\n} catch ( e ) { }\ntry {\nnowebrtc.js\n} catch ( e ) { }\n".to_owned();
            assert_eq!(out, expected);
        }

        let out = cfcache.hostname_cosmetic_resources(&resources, "c.g.cosmetic.net", false);
        expected.injected_script = "try {\nwindow.open-defuser.js\n} catch ( e ) { }\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "d.g.cosmetic.net", false);
        expected.injected_script = "".to_owned();
        assert_eq!(out, expected);
    }

    #[test]
    fn remove_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.element:remove()",
            "sub.test.example.com#@#.element:remove()",
            "a1.sub.example.com##.element",
            "a2.sub.example.com##.element:remove()",
            "a3.example.com##.element:remove()",
        ]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.test.example.com", false);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a1.sub.example.com", false);
        expected.hide_selectors.insert(".element".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.example.com", false);
        expected.hide_selectors.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::Remove),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a2.sub.example.com", false);
        expected.procedural_actions.clear();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a3.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::Remove),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn remove_attr_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.element:remove-attr(style)",
            "sub.test.example.com#@#.element:remove-attr(style)",
            "a1.sub.example.com##.element",
            "a2.sub.example.com##.element:remove-attr(src)",
            "a3.example.com##.element:remove-attr(src)",
        ]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.test.example.com", false);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a1.sub.example.com", false);
        expected.hide_selectors.insert(".element".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.example.com", false);
        expected.hide_selectors.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveAttr("style".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a2.sub.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveAttr("src".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a3.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveAttr("src".to_string())),
            })
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveAttr("style".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn remove_class_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.element:remove-class(overlay)",
            "sub.test.example.com#@#.element:remove-class(overlay)",
            "a1.sub.example.com##.element",
            "a2.sub.example.com##.element:remove-class(banner)",
            "a3.example.com##.element:remove-class(banner)",
        ]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.example.com", false);
        let mut expected = UrlSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "sub.test.example.com", false);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a1.sub.example.com", false);
        expected.hide_selectors.insert(".element".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "test.example.com", false);
        expected.hide_selectors.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveClass("overlay".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a2.sub.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveClass("banner".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources(&resources, "a3.example.com", false);
        expected.procedural_actions.clear();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveClass("banner".to_string())),
            })
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(".element".to_string())],
                action: Some(CosmeticFilterAction::RemoveClass("overlay".to_string())),
            })
            .unwrap(),
        );
        assert_eq!(out, expected);
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn procedural_actions() {
        let cfcache = cache_from_rules(vec![
            "example.com##div:has(video):remove()",
            "example.com##div:has-text(Ad):remove()",
            "example.com##div:has-text(Sponsored) > p",
            "example.com##div:has-text(Cookie) > p:remove-class(overlay)",
        ]);
        let resources = ResourceStorage::default();

        let out = cfcache.hostname_cosmetic_resources(&resources, "example.com", false);
        let mut expected = UrlSpecificResources::empty();
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![CosmeticFilterOperator::CssSelector(
                    "div:has(video)".to_string(),
                )],
                action: Some(CosmeticFilterAction::Remove),
            })
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![
                    CosmeticFilterOperator::CssSelector("div".to_string()),
                    CosmeticFilterOperator::HasText("Ad".to_string()),
                ],
                action: Some(CosmeticFilterAction::Remove),
            })
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![
                    CosmeticFilterOperator::CssSelector("div".to_string()),
                    CosmeticFilterOperator::HasText("Cookie".to_string()),
                    CosmeticFilterOperator::CssSelector(" > p".to_string()),
                ],
                action: Some(CosmeticFilterAction::RemoveClass("overlay".to_string())),
            })
            .unwrap(),
        );
        expected.procedural_actions.insert(
            serde_json::to_string(&ProceduralOrActionFilter {
                selector: vec![
                    CosmeticFilterOperator::CssSelector("div".to_string()),
                    CosmeticFilterOperator::HasText("Sponsored".to_string()),
                    CosmeticFilterOperator::CssSelector(" > p".to_string()),
                ],
                action: None,
            })
            .unwrap(),
        );
        assert_eq!(out, expected);
    }

    /// Avoid impossible type inference for type parameter `impl AsRef<str>`
    const EMPTY: &[&str] = &[];

    #[test]
    fn matching_hidden_class_id_selectors() {
        let rules = [
            "##.a-class",
            "###simple-id",
            "##.a-class .with .children",
            "##.children .including #simple-id",
            "##a.a-class",
        ];
        let cfcache = CosmeticFilterCache::from_rules(
            rules
                .iter()
                .map(|r| CosmeticFilter::parse(r, false, Default::default()).unwrap())
                .collect::<Vec<_>>(),
        );

        let out = cfcache.hidden_class_id_selectors(["with"], EMPTY, &HashSet::default());
        assert_eq!(out, Vec::<String>::new());

        let out = cfcache.hidden_class_id_selectors(EMPTY, ["with"], &HashSet::default());
        assert_eq!(out, Vec::<String>::new());

        let out = cfcache.hidden_class_id_selectors(EMPTY, ["a-class"], &HashSet::default());
        assert_eq!(out, Vec::<String>::new());

        let out = cfcache.hidden_class_id_selectors(["simple-id"], EMPTY, &HashSet::default());
        assert_eq!(out, Vec::<String>::new());

        let out = cfcache.hidden_class_id_selectors(["a-class"], EMPTY, &HashSet::default());
        assert_eq!(out, [".a-class", ".a-class .with .children"]);

        let out =
            cfcache.hidden_class_id_selectors(["children", "a-class"], EMPTY, &HashSet::default());
        assert_eq!(
            out,
            [
                ".children .including #simple-id",
                ".a-class",
                ".a-class .with .children",
            ]
        );

        let out = cfcache.hidden_class_id_selectors(EMPTY, ["simple-id"], &HashSet::default());
        assert_eq!(out, ["#simple-id"]);

        let out = cfcache.hidden_class_id_selectors(
            ["children", "a-class"],
            ["simple-id"],
            &HashSet::default(),
        );
        assert_eq!(
            out,
            [
                ".children .including #simple-id",
                ".a-class",
                ".a-class .with .children",
                "#simple-id",
            ]
        );
    }

    #[test]
    fn class_id_exceptions() {
        let rules = [
            "##.a-class",
            "###simple-id",
            "##.a-class .with .children",
            "##.children .including #simple-id",
            "##a.a-class",
            "example.*#@#.a-class",
            "~test.com###test-element",
        ];
        let cfcache = CosmeticFilterCache::from_rules(
            rules
                .iter()
                .map(|r| CosmeticFilter::parse(r, false, Default::default()).unwrap())
                .collect::<Vec<_>>(),
        );
        let resources = ResourceStorage::default();
        let exceptions = cfcache
            .hostname_cosmetic_resources(&resources, "example.co.uk", false)
            .exceptions;

        let out = cfcache.hidden_class_id_selectors(["a-class"], EMPTY, &exceptions);
        assert_eq!(out, [".a-class .with .children"]);

        let out =
            cfcache.hidden_class_id_selectors(["children", "a-class"], ["simple-id"], &exceptions);
        assert_eq!(
            out,
            [
                ".children .including #simple-id",
                ".a-class .with .children",
                "#simple-id",
            ]
        );

        let out = cfcache.hidden_class_id_selectors(EMPTY, ["test-element"], &exceptions);
        assert_eq!(out, ["#test-element"]);

        let exceptions = cfcache
            .hostname_cosmetic_resources(&resources, "a1.test.com", false)
            .exceptions;

        let out = cfcache.hidden_class_id_selectors(["a-class"], EMPTY, &exceptions);
        assert_eq!(out, [".a-class", ".a-class .with .children"]);

        let out =
            cfcache.hidden_class_id_selectors(["children", "a-class"], ["simple-id"], &exceptions);
        assert_eq!(
            out,
            [
                ".children .including #simple-id",
                ".a-class",
                ".a-class .with .children",
                "#simple-id",
            ]
        );

        let out = cfcache.hidden_class_id_selectors(EMPTY, ["test-element"], &exceptions);
        assert_eq!(out, Vec::<String>::new());
    }

    #[test]
    fn misc_generic_exceptions() {
        let rules = [
            "##a[href=\"bad.com\"]",
            "##div > p",
            "##a[href=\"notbad.com\"]",
            "example.com#@#div > p",
            "~example.com##a[href=\"notbad.com\"]",
        ];
        let cfcache = CosmeticFilterCache::from_rules(
            rules
                .iter()
                .map(|r| CosmeticFilter::parse(r, false, Default::default()).unwrap())
                .collect::<Vec<_>>(),
        );
        let resources = ResourceStorage::default();

        let hide_selectors = cfcache
            .hostname_cosmetic_resources(&resources, "test.com", false)
            .hide_selectors;
        let mut expected_hides = HashSet::new();
        expected_hides.insert("a[href=\"bad.com\"]".to_owned());
        expected_hides.insert("div > p".to_owned());
        expected_hides.insert("a[href=\"notbad.com\"]".to_owned());
        assert_eq!(hide_selectors, expected_hides);

        let hide_selectors = cfcache
            .hostname_cosmetic_resources(&resources, "example.com", false)
            .hide_selectors;
        let mut expected_hides = HashSet::new();
        expected_hides.insert("a[href=\"bad.com\"]".to_owned());
        assert_eq!(hide_selectors, expected_hides);
    }

    #[test]
    fn apply_to_tld() {
        use crate::resources::ResourceType;

        // toolforge.org and github.io are examples of TLDs with multiple segments. These rules
        // should still be parsed correctly and applied on corresponding subdomains.
        let rules = [
            "toolforge.org##+js(abort-on-property-read, noAdBlockers)",
            "github.io##div.adToBlock",
        ];
        let cfcache = CosmeticFilterCache::from_rules(
            rules
                .iter()
                .map(|r| CosmeticFilter::parse(r, false, Default::default()).unwrap())
                .collect::<Vec<_>>(),
        );
        let resources = ResourceStorage::from_resources([Resource {
            name: "abort-on-property-read.js".into(),
            aliases: vec!["aopr".to_string()],
            kind: ResourceType::Template,
            content: BASE64_STANDARD.encode("abort-on-property-read.js, {{1}}"),
            dependencies: vec![],
            permission: Default::default(),
        }]);

        let injected_script = cfcache
            .hostname_cosmetic_resources(&resources, "antonok.toolforge.org", false)
            .injected_script;
        assert_eq!(
            injected_script,
            "try {\nabort-on-property-read.js, noAdBlockers\n} catch ( e ) { }\n"
        );

        let hide_selectors = cfcache
            .hostname_cosmetic_resources(&resources, "antonok.github.io", false)
            .hide_selectors;
        let mut expected_hides = HashSet::new();
        expected_hides.insert("div.adToBlock".to_owned());
        assert_eq!(hide_selectors, expected_hides);
    }
}
