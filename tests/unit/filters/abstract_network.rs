use super::*;

#[test]
fn parse_pipe_delimited_domains_works() {
    let values = parse_pipe_delimited_domains("google.*|gstatic.com|~example.it").unwrap();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0], (true, "google.*"));
    assert_eq!(values[1], (true, "gstatic.com"));
    assert_eq!(values[2], (false, "example.it"));
}

#[test]
fn parse_pipe_delimited_domains_strips_regex() {
    assert!(parse_pipe_delimited_domains("/^foo/").is_err());
    let values = parse_pipe_delimited_domains("/^foo/|bar.com").unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0], (true, "bar.com"));
}

/// Returns the `$replace` value of `line` and the number of its options.
fn replace_value_and_option_count(line: &str) -> (String, usize) {
    let options = AbstractNetworkFilter::parse(line).unwrap().options.unwrap();
    let value = options.iter().find_map(|option| match option {
        NetworkFilterOption::Replace(value) => Some(value.to_string()),
        _ => None,
    });
    (value.expect("no replace option"), options.len())
}

#[test]
fn replace_value_is_kept_whole_and_other_options_are_still_split() {
    for (line, value, option_count) in [
        (
            r"||example.com^$replace=/foo\,bar/\$1/,script",
            r"/foo\,bar/\$1/",
            2,
        ),
        (
            r"||example.com^$replace=/a\,b\,c/\$1\$2/,script",
            r"/a\,b\,c/\$1\$2/",
            2,
        ),
        (
            r"||example.com^$script,replace=/a\,b/c/i,domain=x.org",
            r"/a\,b/c/i",
            3,
        ),
        // An escaped backslash does not escape the options separator.
        (r"||example.com/a\\$replace=/x/y/", "/x/y/", 1),
        // Unescaped commas inside the value, and options before and after it.
        (
            "||example.com^$script,replace=/a, 1/b();}, 2/gms,domain=x.org",
            "/a, 1/b();}, 2/gms",
            3,
        ),
        ("||example.com^$replace=/a/b/,script,important", "/a/b/", 3),
        // Unescaped `$` inside the value.
        (
            "||example.com^$script,replace=/(a)(b)/$2$1/g,domain=x.org",
            "/(a)(b)/$2$1/g",
            3,
        ),
        // A JSONPath query ends at the first unescaped comma.
        (
            "||example.com^$xhr,replace=json:$..a,domain=x.org",
            "json:$..a",
            3,
        ),
        (
            r"||example.com^$replace=jsonl:$['a'\,'b'],script",
            r"jsonl:$['a'\,'b']",
            2,
        ),
    ] {
        assert_eq!(
            replace_value_and_option_count(line),
            (value.to_string(), option_count),
            "{line}"
        );
    }
}

#[test]
fn parse_empty_replace_only_for_exception() {
    let parsed = AbstractNetworkFilter::parse("@@||example.com^$replace").unwrap();
    assert!(parsed.exception);
    assert!(matches!(
        parsed.options.unwrap().as_slice(),
        [NetworkFilterOption::Replace("")]
    ));

    assert_eq!(
        AbstractNetworkFilter::parse("||example.com^$replace").err(),
        Some(NetworkFilterError::EmptyReplace)
    );
}

#[test]
fn negated_replace_is_rejected() {
    assert_eq!(
        AbstractNetworkFilter::parse("||example.com^$~replace=/a/b/").err(),
        Some(NetworkFilterError::NegatedReplace)
    );
}
#[test]
fn escaped_trailing_dollar_is_part_of_the_pattern() {
    assert!(
        AbstractNetworkFilter::parse(r"/banner\$")
            .unwrap()
            .options
            .is_none()
    );
}

#[test]
fn unescaped_dollar_outside_a_replace_line_still_starts_the_options() {
    assert!(AbstractNetworkFilter::parse("||example.com/a$1").is_err());
}
