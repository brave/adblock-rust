#[cfg(test)]
mod extract_function_name_tests {
    use super::super::extract_function_name;

    #[test]
    fn test_extract_function_name() {
        assert_eq!(extract_function_name("function test() {}"), Some("test"));
        assert_eq!(extract_function_name("function $() {}"), Some("$"));
        assert_eq!(extract_function_name("function _() {}"), Some("_"));
        assert_eq!(extract_function_name("function ಠ_ಠ() {}"), Some("ಠ_ಠ"));
        assert_eq!(extract_function_name("function\ntest\n(\n)\n{\n}"), Some("test"));
        assert_eq!(extract_function_name("function\ttest\t(\t)\t{\t}"), Some("test"));
        assert_eq!(extract_function_name("function test() { (function inner() {})() }"), Some("test"));
        assert_eq!(extract_function_name("let e = function test() { (function inner() {})() }"), None);
        assert_eq!(extract_function_name("function () { (function inner() {})() }"), None);
    }
}

#[cfg(test)]
mod arg_parsing_util_tests {
    use super::super::*;

    #[test]
    fn test_index_next_unescaped_separator() {
        assert_eq!(index_next_unescaped_separator(r#"``"#, '`'), (Some(0), false));
        assert_eq!(index_next_unescaped_separator(r#"\``"#, '`'), (Some(2), true));
        assert_eq!(index_next_unescaped_separator(r#"\\``"#, '`'), (Some(2), false));
        assert_eq!(index_next_unescaped_separator(r#"\\\``"#, '`'), (Some(4), true));
        assert_eq!(index_next_unescaped_separator(r#"\\\\``"#, '`'), (Some(4), false));
        assert_eq!(index_next_unescaped_separator(r#"\`\\\``"#, '`'), (Some(6), true));
        assert_eq!(index_next_unescaped_separator(r#"\\\`\``"#, '`'), (Some(6), true));
        assert_eq!(index_next_unescaped_separator(r#"\\\`\\``"#, '`'), (Some(6), true));

        assert_eq!(index_next_unescaped_separator(r#"\,test\,"#, ','), (None, true))
    }

    #[test]
    fn test_normalize_arg() {
        assert_eq!(normalize_arg(r#"\`"#, '`'), r#"`"#);
        assert_eq!(normalize_arg(r#"\\\`"#, '`'), r#"\\`"#);
        assert_eq!(normalize_arg(r#"\`\\\`"#, '`'), r#"`\\`"#);
        assert_eq!(normalize_arg(r#"\\\`\`"#, '`'), r#"\\``"#);
        assert_eq!(normalize_arg(r#"\\\`\\`"#, '`'), r#"\\`\\`"#);
    }
}

#[cfg(test)]
mod redirect_storage_tests {
    use super::super::*;
    use crate::resources::MimeType;

    #[test]
    fn get_resource_by_name() {
        let mut storage = ResourceStorage::default();
        storage
            .add_resource(
                Resource::simple("name.js", MimeType::ApplicationJavascript, "resource data"),
            )
            .unwrap();

        assert_eq!(
            storage.get_redirect_resource("name.js"),
            Some(format!("data:application/javascript;base64,{}", base64::encode("resource data"))),
        );
    }

    #[test]
    fn get_resource_by_alias() {
        let mut storage = ResourceStorage::default();
        let mut r = Resource::simple("name.js", MimeType::ApplicationJavascript, "resource data");
        r.aliases.push("alias.js".to_string());
        storage
            .add_resource(r)
            .unwrap();

        assert_eq!(
            storage.get_redirect_resource("alias.js"),
            Some(format!("data:application/javascript;base64,{}", base64::encode("resource data"))),
        );
    }

    #[test]
    fn permissions() {
        let mut storage = ResourceStorage::default();
        let mut r = Resource::simple("name.js", MimeType::ApplicationJavascript, "resource data");
        r.aliases.push("alias.js".to_string());
        r.permission = PermissionMask::from_bits(0b00000001);
        storage
            .add_resource(r)
            .unwrap();

        assert_eq!(
            storage.get_redirect_resource("name.js"),
            None,
        );
        assert_eq!(
            storage.get_redirect_resource("alias.js"),
            None,
        );
    }
}

#[cfg(test)]
mod scriptlet_storage_tests {
    use super::super::*;
    use crate::resources::MimeType;

    #[test]
    fn parse_argslist() {
        let args = parse_scriptlet_args("scriptlet, hello world, foobar").unwrap();
        assert_eq!(args, vec!["scriptlet", "hello world", "foobar"]);
    }

    #[test]
    fn parse_argslist_noargs() {
        let args = parse_scriptlet_args("scriptlet").unwrap();
        assert_eq!(args, vec!["scriptlet"]);
    }

    #[test]
    fn parse_argslist_empty() {
        let args = parse_scriptlet_args("").unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn parse_argslist_commas() {
        let args = parse_scriptlet_args("scriptletname, one\\, two\\, three, four").unwrap();
        assert_eq!(args, vec!["scriptletname", "one, two, three", "four"]);
    }

    #[test]
    fn parse_argslist_badchars() {
        let args = parse_scriptlet_args(
            r##"scriptlet, "; window.location.href = bad.com; , '; alert("you're\, hacked");    ,    \u\r\l(bad.com) "##,
        );
        assert_eq!(args, None);
    }

    #[test]
    fn parse_argslist_quoted() {
        let args = parse_scriptlet_args(r#"debug-scriptlet, 'test', '"test"', "test", "'test'", `test`, '`test`'"#).unwrap();
        assert_eq!(
            args,
            vec![
                r#"debug-scriptlet"#,
                r#"test"#,
                r#""test""#,
                r#"test"#,
                r#"'test'"#,
                r#"test"#,
                r#"`test`"#,
            ],
        );
        let args = parse_scriptlet_args(r#"debug-scriptlet, 'test,test', '', "", ' ', ' test '"#).unwrap();
        assert_eq!(
            args,
            vec![
                r#"debug-scriptlet"#,
                r#"test,test"#,
                r#""#,
                r#""#,
                r#" "#,
                r#" test "#,
            ],
        );
        let args = parse_scriptlet_args(r#"debug-scriptlet, test\,test, test\test, "test\test", 'test\test', "#).unwrap();
        assert_eq!(
            args,
            vec![
                r#"debug-scriptlet"#,
                r#"test,test"#,
                r#"test\test"#,
                r#"test\test"#,
                r#"test\test"#,
                r#""#,
            ],
        );
        let args = parse_scriptlet_args(r#"debug-scriptlet, "test"#);
        assert_eq!(args, None);
        let args = parse_scriptlet_args(r#"debug-scriptlet, 'test'"test""#);
        assert_eq!(args, None);
    }

    #[test]
    fn parse_argslist_trailing_escaped_comma() {
        let args = parse_scriptlet_args(r#"remove-node-text, script, \,mr=function(r\,"#).unwrap();
        assert_eq!(args, vec!["remove-node-text", "script", ",mr=function(r,"]);
    }

    #[test]
    fn get_patched_scriptlets() {
        let resources = ResourceStorage::from_resources([
            Resource {
                name: "greet.js".to_string(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode("console.log('Hello {{1}}, my name is {{2}}')"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "alert.js".to_owned(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode("alert('{{1}}')"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "blocktimer.js".to_owned(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode("setTimeout(blockAds, {{1}})"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "null.js".to_owned(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode("(()=>{})()"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "set-local-storage-item.js".to_owned(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode(r#"{{1}} that dollar signs in {{2}} are untouched"#),
                dependencies: vec![],
                permission: Default::default(),
            },
        ]);

        assert_eq!(
            resources.get_scriptlet_resources([("greet, world, adblock-rust", Default::default())]),
            "try {\nconsole.log('Hello world, my name is adblock-rust')\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([("alert, All systems are go!! ", Default::default())]),
            "try {\nalert('All systems are go!!')\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([("alert, Uh oh\\, check the logs...", Default::default())]),
            "try {\nalert('Uh oh, check the logs...')\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([(r#"alert, this has "quotes""#, Default::default())]),
            "try {\nalert('this has \\\"quotes\\\"')\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([("blocktimer, 3000", Default::default())]),
            "try {\nsetTimeout(blockAds, 3000)\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([("null", Default::default())]),
            "try {\n(()=>{})()\n} catch ( e ) { }\n"
        );
        assert_eq!(
            resources.get_scriptlet_resources([("null, null", Default::default())]),
            "try {\n(()=>{})()\n} catch ( e ) { }\n",
        );
        assert_eq!(
            resources.get_scriptlet_resources([("greet, everybody", Default::default())]),
            "try {\nconsole.log('Hello everybody, my name is {{2}}')\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resource("unit-testing", Default::default(), &mut vec![]),
            Err(ScriptletResourceError::NoMatchingScriptlet),
        );
        assert_eq!(
            resources.get_scriptlet_resource("", Default::default(), &mut vec![]),
            Err(ScriptletResourceError::MissingScriptletName),
        );

        assert_eq!(
            resources.get_scriptlet_resources([("set-local-storage-item, Test, $remove$", Default::default())]),
            "try {\nTest that dollar signs in $remove$ are untouched\n} catch ( e ) { }\n",
        );
    }

    #[test]
    fn parse_template_file_format() {
        let resources = ResourceStorage::from_resources([
            Resource {
                name: "abort-current-inline-script.js".into(),
                aliases: vec!["acis.js".into()],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("(function() {alert(\"hi\");})();"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "abort-on-property-read.js".into(),
                aliases: vec!["aopr.js".into()],
                kind: ResourceType::Template,
                content: base64::encode("(function() {confirm(\"Do you want to {{1}}?\");})();"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "googletagservices_gpt.js".into(),
                aliases: vec!["googletagservices.com/gpt.js".into(), "googletagservices-gpt".into()],
                kind: ResourceType::Template,
                content: base64::encode("function gpt(a1 = '', a2 = '') {console.log(a1, a2)}"),
                dependencies: vec![],
                permission: Default::default(),
            },
        ]);

        assert_eq!(
            resources.get_scriptlet_resources([("aopr, code", Default::default())]),
            "try {\n(function() {confirm(\"Do you want to code?\");})();\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("abort-on-property-read, write tests", Default::default())]),
            "try {\n(function() {confirm(\"Do you want to write tests?\");})();\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("abort-on-property-read.js, block advertisements", Default::default())]),
            "try {\n(function() {confirm(\"Do you want to block advertisements?\");})();\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("acis", Default::default())]),
            "try {\n(function() {alert(\"hi\");})();\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("acis.js", Default::default())]),
            "try {\n(function() {alert(\"hi\");})();\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("googletagservices_gpt.js", Default::default())]),
            "function gpt(a1 = '', a2 = '') {console.log(a1, a2)}\ntry {\ngpt()\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("googletagservices_gpt, test1", Default::default())]),
            "function gpt(a1 = '', a2 = '') {console.log(a1, a2)}\ntry {\ngpt(\"test1\")\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resources([("googletagservices.com/gpt, test1, test2", Default::default())]),
            "function gpt(a1 = '', a2 = '') {console.log(a1, a2)}\ntry {\ngpt(\"test1\", \"test2\")\n} catch ( e ) { }\n",
        );

        assert_eq!(
            resources.get_scriptlet_resource(r#"googletagservices.com/gpt.js, t"es't1, $te\st2$"#, Default::default(), &mut vec![]),
            Ok(r#"gpt("t\"es't1", "$te\\st2$")"#.to_owned()),
        );

        // The alias does not have a `.js` extension, so it cannot be used for a scriptlet
        // injection (only as a redirect resource).
        assert_eq!(
            resources.get_scriptlet_resource(r#"googletagservices-gpt, t"es't1, te\st2"#, Default::default(), &mut vec![]),
            Err(ScriptletResourceError::NoMatchingScriptlet),
        );

        // Object-style injection
        assert_eq!(
            resources.get_scriptlet_resource(r#"googletagservices.com/gpt, { "test": true }"#, Default::default(), &mut vec![]),
            Err(ScriptletResourceError::ScriptletArgObjectSyntaxUnsupported),
        );
    }

    /// Currently, only 9 template arguments are supported - but reaching that limit should not
    /// cause a panic.
    #[test]
    fn patch_argslist_many_args() {
        let resources = ResourceStorage::from_resources([
            Resource {
                name: "abort-current-script.js".into(),
                aliases: vec!["acs.js".into()],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("{{1}} {{2}} {{3}} {{4}} {{5}} {{6}} {{7}} {{8}} {{9}} {{10}} {{11}} {{12}}"),
                dependencies: vec![],
                permission: Default::default(),
            },
        ]);

        let args = parse_scriptlet_args("acs, this, probably, is, going, to, break, brave, and, crash, it, instead, of, ignoring, it").unwrap();
        assert_eq!(args, vec!["acs", "this", "probably", "is", "going", "to", "break", "brave", "and", "crash", "it", "instead", "of", "ignoring", "it"]);

        assert_eq!(
            resources.get_scriptlet_resources([("acs, this, probably, is, going, to, break, brave, and, crash, it, instead, of, ignoring, it", Default::default())]),
            "try {\nthis probably is going to break brave and crash {{10}} {{11}} {{12}}\n} catch ( e ) { }\n",
        );
    }

    #[test]
    fn permissions() {
        const PERM01: PermissionMask = PermissionMask::from_bits(0b00000001);
        const PERM10: PermissionMask = PermissionMask::from_bits(0b00000010);
        const PERM11: PermissionMask = PermissionMask::from_bits(0b00000011);
        let resources = ResourceStorage::from_resources([
            Resource::simple("default-perms.js", MimeType::ApplicationJavascript, "default-perms"),
            Resource {
                name: "perm0.js".into(),
                aliases: vec!["0.js".to_string()],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("perm0"),
                dependencies: vec![],
                permission: PERM01,
            },
            Resource {
                name: "perm1.js".into(),
                aliases: vec!["1.js".to_string()],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("perm1"),
                dependencies: vec![],
                permission: PERM10,
            },
            Resource {
                name: "perm10.js".into(),
                aliases: vec!["10.js".to_string()],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("perm10"),
                dependencies: vec![],
                permission: PERM11,
            },
        ]);

        fn test_perm(resources: &ResourceStorage, perm: PermissionMask, expect_ok: &[&str], expect_fail: &[&str]) {
            for ident in expect_ok {
                if ident.len() > 2 {
                    assert_eq!(
                        resources.get_scriptlet_resources([(*ident, perm)]),
                        format!("try {{\n{}\n}} catch ( e ) {{ }}\n", ident),
                    );
                } else {
                    assert_eq!(
                        resources.get_scriptlet_resources([(*ident, perm)]),
                        format!("try {{\nperm{}\n}} catch ( e ) {{ }}\n", ident),
                    );
                }
            }

            for ident in expect_fail {
                assert_eq!(
                    resources.get_scriptlet_resource(ident, perm, &mut vec![]),
                    Err(ScriptletResourceError::InsufficientPermissions),
                );
            }
        }

        test_perm(&resources, Default::default(), &["default-perms"], &["perm0", "perm1", "perm10", "0", "1", "10"]);
        test_perm(&resources, PERM01, &["default-perms", "perm0", "0"], &["perm1", "perm10", "1", "10"]);
        test_perm(&resources, PERM10, &["default-perms", "perm1", "1"], &["perm0", "perm10", "0", "10"]);
        test_perm(&resources, PERM11, &["default-perms", "perm0", "perm1", "perm10", "0", "1", "10"], &[]);
    }

    #[test]
    fn dependencies() {
        const PERM01: PermissionMask = PermissionMask::from_bits(0b00000001);
        let resources = ResourceStorage::from_resources([
            Resource::simple("simple.fn", MimeType::FnJavascript, "simple"),
            Resource {
                name: "permissioned.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("permissioned"),
                dependencies: vec!["a.fn".to_string(), "common.fn".to_string()],
                permission: PERM01,
            },
            Resource {
                name: "a.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("a"),
                dependencies: vec!["common.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "b.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("b"),
                dependencies: vec!["common.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "common.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("common"),
                dependencies: vec![],
                permission: Default::default(),
            },
            Resource {
                name: "test.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("function test() {}"),
                dependencies: vec!["permissioned.fn".to_string(), "a.fn".to_string(), "b.fn".to_string(), "common.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "deploop1.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("deploop1"),
                dependencies: vec!["deploop1.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "deploop2a.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("deploop2a"),
                dependencies: vec!["deploop2b.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "deploop2b.fn".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::FnJavascript),
                content: base64::encode("deploop2b"),
                dependencies: vec!["deploop2a.fn".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "test-wrapper.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("function testWrapper() { test(arguments) }"),
                dependencies: vec!["test.js".to_string()],
                permission: Default::default(),
            },
            Resource {
                name: "shared.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::ApplicationJavascript),
                content: base64::encode("function shared() { }"),
                dependencies: vec!["a.fn".to_string(), "b.fn".to_string()],
                permission: Default::default(),
            },
        ]);

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("common.fn", &mut deps, Default::default()), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["common.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("a.fn", &mut deps, Default::default()), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["a.fn", "common.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("b.fn", &mut deps, Default::default()), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["b.fn", "common.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("permissioned.fn", &mut deps, Default::default()), Err(ScriptletResourceError::InsufficientPermissions));
        }
        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("permissioned.fn", &mut deps, PERM01), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["permissioned.fn", "a.fn", "common.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("test.js", &mut deps, Default::default()), Err(ScriptletResourceError::InsufficientPermissions));
        }
        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("test.js", &mut deps, PERM01), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["test.js", "permissioned.fn", "a.fn", "common.fn", "b.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("deploop1.fn", &mut deps, Default::default()), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["deploop1.fn"]);
        }

        {
            let mut deps = vec![];
            assert_eq!(resources.recursive_dependencies("deploop2a.fn", &mut deps, Default::default()), Ok(()));
            assert_eq!(deps.iter().map(|dep| dep.name.to_string()).collect::<Vec<_>>(), vec!["deploop2a.fn", "deploop2b.fn"]);
        }

        assert_eq!(resources.get_scriptlet_resources([]), "");

        assert_eq!(resources.get_scriptlet_resources([("test, arg1, arg2", Default::default())]), "");

        assert_eq!(resources.get_scriptlet_resources([("test, arg1, arg2", PERM01)]), "permissioned\na\ncommon\nb\nfunction test() {}\ntry {\ntest(\"arg1\", \"arg2\")\n} catch ( e ) { }\n");

        // Note: `test` still gets inserted as a dependency before it becomes apparent that
        // `permissioned` is not authorized. However, this shouldn't have much detrimental effect.
        assert_eq!(resources.get_scriptlet_resources([("test-wrapper", Default::default())]), "function test() {}\n");
        assert_eq!(resources.get_scriptlet_resources([("test-wrapper", PERM01)]), "function test() {}\npermissioned\na\ncommon\nb\nfunction testWrapper() { test(arguments) }\ntry {\ntestWrapper()\n} catch ( e ) { }\n");

        assert_eq!(resources.get_scriptlet_resources([("test", PERM01), ("test-wrapper", PERM01)]), "permissioned\na\ncommon\nb\nfunction test() {}\nfunction testWrapper() { test(arguments) }\ntry {\ntest()\n} catch ( e ) { }\ntry {\ntestWrapper()\n} catch ( e ) { }\n");

        assert_eq!(resources.get_scriptlet_resources([("shared, argument", Default::default())]), "a\ncommon\nb\nfunction shared() { }\ntry {\nshared(\"argument\")\n} catch ( e ) { }\n");
        assert_eq!(resources.get_scriptlet_resources([("test, 1", PERM01), ("test-wrapper, 2", PERM01), ("shared, 3", Default::default())]), "permissioned\na\ncommon\nb\nfunction test() {}\nfunction testWrapper() { test(arguments) }\nfunction shared() { }\ntry {\ntest(\"1\")\n} catch ( e ) { }\ntry {\ntestWrapper(\"2\")\n} catch ( e ) { }\ntry {\nshared(\"3\")\n} catch ( e ) { }\n");
    }
}
