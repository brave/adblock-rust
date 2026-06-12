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
