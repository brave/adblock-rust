#[cfg(test)]
mod tests {
    use super::super::*;

    fn build_request(
        raw_type: &str,
        url: &str,
        schema: &str,
        hostname: &str,
        domain: &str,
        source_hostname: &str,
        source_domain: &str,
    ) -> Request {
        let third_party = source_domain != domain;

        Request::from_detailed_parameters(
            raw_type,
            url,
            schema,
            hostname,
            source_hostname,
            third_party,
            url.to_string(),
        )
    }

    #[test]
    fn new_works() {
        let simple_example = build_request(
            "document",
            "https://example.com/ad",
            "https",
            "example.com",
            "example.com",
            "example.com",
            "example.com",
        );
        assert!(simple_example.is_https);
        assert!(simple_example.is_supported);
        assert!(!simple_example.is_third_party);
        assert_eq!(simple_example.request_type, RequestType::Document);
        assert_eq!(
            simple_example.source_hostname_hashes,
            Some(vec![
                utils::fast_hash("example.com"),
                utils::fast_hash("com")
            ]),
        );

        let unsupported_example = build_request(
            "document",
            "file://example.com/ad",
            "file",
            "example.com",
            "example.com",
            "example.com",
            "example.com",
        );
        assert!(!unsupported_example.is_https);
        assert!(!unsupported_example.is_http);
        assert!(!unsupported_example.is_supported);

        let first_party = build_request(
            "document",
            "https://subdomain.example.com/ad",
            "https",
            "subdomain.example.com",
            "example.com",
            "example.com",
            "example.com",
        );
        assert!(first_party.is_https);
        assert!(first_party.is_supported);
        assert!(!first_party.is_third_party);

        let third_party = build_request(
            "document",
            "https://subdomain.anotherexample.com/ad",
            "https",
            "subdomain.anotherexample.com",
            "anotherexample.com",
            "example.com",
            "example.com",
        );
        assert!(third_party.is_https);
        assert!(third_party.is_supported);
        assert!(third_party.is_third_party);

        let websocket = build_request(
            "document",
            "wss://subdomain.anotherexample.com/ad",
            "wss",
            "subdomain.anotherexample.com",
            "anotherexample.com",
            "example.com",
            "example.com",
        );
        assert!(!websocket.is_https);
        assert!(!websocket.is_https);
        assert!(websocket.is_supported);
        assert!(websocket.is_third_party);
        assert_eq!(websocket.request_type, RequestType::Websocket);

        let assumed_https = build_request(
            "document",
            "//subdomain.anotherexample.com/ad",
            "",
            "subdomain.anotherexample.com",
            "anotherexample.com",
            "example.com",
            "example.com",
        );
        assert!(assumed_https.is_https);
        assert!(!assumed_https.is_http);
        assert!(assumed_https.is_supported);
    }

    fn tokenize(tokens: &[&str], extra_tokens: &[utils::Hash]) -> Vec<utils::Hash> {
        let mut tokens: Vec<_> = tokens.iter().map(|t| utils::fast_hash(t)).collect();
        tokens.extend(extra_tokens);
        tokens
    }

    #[test]
    fn tokens_works() {
        let simple_example = build_request(
            "document",
            "https://subdomain.example.com/ad",
            "https",
            "subdomain.example.com",
            "example.com",
            "subdomain.example.com",
            "example.com",
        );
        assert_eq!(
            simple_example
                .source_hostname_hashes
                .as_ref()
                .unwrap()
                .as_slice(),
            tokenize(&["subdomain.example.com", "example.com", "com",], &[]).as_slice()
        );
        let tokens = simple_example.get_tokens();
        assert_eq!(
            tokens.as_slice(),
            tokenize(&["https", "subdomain", "example", "com", "ad"], &[0]).as_slice()
        )
    }

    #[test]
    fn parses_urls() {
        let parsed = Request::new(
            "https://subdomain.example.com/ad",
            "https://example.com/",
            "document",
        )
        .unwrap();
        assert!(parsed.is_https);
        assert!(parsed.is_supported);
        assert!(!parsed.is_third_party);
        assert_eq!(parsed.request_type, RequestType::Document);

        // assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.hostname, "subdomain.example.com");

        // assert_eq!(parsed.source_domain, "example.com");
        assert_eq!(
            parsed.source_hostname_hashes,
            Some(vec![
                utils::fast_hash("example.com"),
                utils::fast_hash("com")
            ]),
        );
        // assert_eq!(parsed.source_hostname, "example.com");

        let bad_url = Request::new(
            "subdomain.example.com/ad",
            "https://example.com/",
            "document",
        );
        assert_eq!(bad_url.err(), Some(RequestError::HostnameParseError));
    }

    #[test]
    fn fuzzing_errors() {
        {
            let parsed = Request::new("https://߶", "https://example.com", "other");
            assert!(parsed.is_ok());
        }
        {
            let parsed = Request::new(
                &format!("https://{}", std::str::from_utf8(&[9, 9, 64]).unwrap()),
                "https://example.com",
                "other",
            );
            assert!(parsed.is_err());
        }
    }
}
