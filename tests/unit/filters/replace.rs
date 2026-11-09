use super::*;

fn regex(pattern: &str, replacement: &str, flags: &str) -> ReplaceRule {
    ReplaceRule::Regex {
        pattern: pattern.to_string(),
        replacement: replacement.to_string(),
        flags: flags.to_string(),
    }
}

#[test]
fn splits_pattern_replacement_and_flags() {
    assert_eq!(
        ReplaceRule::parse("/foo(bar)/$1baz/gi").unwrap(),
        regex("foo(bar)", "$1baz", "gi")
    );
}

#[test]
fn unescapes_delimiters_and_replace_literals() {
    assert_eq!(
        ReplaceRule::parse(r"/a\/b\,c/\$1\,/").unwrap(),
        regex("a/b,c", "$1,", "")
    );
}

#[test]
fn does_not_compile_or_validate_regex_syntax() {
    assert_eq!(
        ReplaceRule::parse("/(?<=a)b/$&/gimsu").unwrap(),
        regex("(?<=a)b", "$&", "gimsu")
    );
}

#[test]
fn rejects_unknown_flags() {
    for value in ["/a/b/z", "/a/b/y", "/a/b/G", "/a/b/1", "/a/b/unknown"] {
        assert_eq!(
            ReplaceRule::parse(value),
            Err(ReplaceOptionParseError),
            "{value:?}"
        );
    }
}

#[test]
fn allows_unescaped_slashes_in_replacement() {
    assert_eq!(
        ReplaceRule::parse(
            r#"/<div\b[^>]*?\sid\s*=\s*["']donate["'][^>]*>[\s\S]*?<\/div>/<div>Works</div>/"#,
        )
        .unwrap(),
        regex(
            r#"<div\b[^>]*?\sid\s*=\s*["']donate["'][^>]*>[\s\S]*?</div>"#,
            "<div>Works</div>",
            ""
        )
    );

    assert_eq!(
        ReplaceRule::parse(r"/a/<b>\/</b>/g").unwrap(),
        regex("a", "<b>/</b>", "g")
    );
}

#[test]
fn reports_options_without_both_delimiters() {
    for value in ["", "foo/bar/", "/foo", "/foo/bar", "abc", "jsonx:$.a"] {
        assert_eq!(
            ReplaceRule::parse(value),
            Err(ReplaceOptionParseError),
            "{value:?}"
        );
    }
}

#[test]
fn rejects_an_empty_pattern() {
    assert_eq!(ReplaceRule::parse("//x/"), Err(ReplaceOptionParseError));
}

#[test]
fn parses_json_queries() {
    assert_eq!(
        ReplaceRule::parse("json:$..adPlacements").unwrap(),
        ReplaceRule::Json {
            query: "$..adPlacements".to_string()
        }
    );
    assert_eq!(
        ReplaceRule::parse(r"jsonl:$['a'\,'b']").unwrap(),
        ReplaceRule::JsonLines {
            query: "$['a','b']".to_string()
        }
    );
    for value in ["json:", "jsonl:"] {
        assert_eq!(
            ReplaceRule::parse(value),
            Err(ReplaceOptionParseError),
            "{value:?}"
        );
    }
}
