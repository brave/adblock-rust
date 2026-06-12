use super::*;

#[test]
fn parse_domain_option_values_plain_and_entity() {
    let values = parse_domain_option_values("google.*|gstatic.com|~example.it").unwrap();
    assert_eq!(values.len(), 3);
    assert_eq!(
        values[0],
        ParsedDomainValue {
            included: true,
            kind: DomainValueKind::Entity,
            value: "google",
            raw: "google.*",
        }
    );
    assert_eq!(
        values[1],
        ParsedDomainValue {
            included: true,
            kind: DomainValueKind::Plain,
            value: "gstatic.com",
            raw: "gstatic.com",
        }
    );
    assert_eq!(
        values[2],
        ParsedDomainValue {
            included: false,
            kind: DomainValueKind::Plain,
            value: "example.it",
            raw: "example.it",
        }
    );
}

#[test]
fn parse_domain_option_values_strips_regex() {
    assert!(parse_domain_option_values("/^foo/").is_err());
    let values = parse_domain_option_values("/^foo/|bar.com").unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value, "bar.com");
}
