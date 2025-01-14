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
        assert_eq!(simple_example.is_https, true);
        assert_eq!(simple_example.is_supported, true);
        assert_eq!(simple_example.is_third_party, false);
        assert_eq!(simple_example.request_type, RequestType::Document);
        assert_eq!(
            simple_example.source_hostname_hashes.unwrap().as_slice(),
            vec![utils::fast_hash("example.com"), utils::fast_hash("com")],
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
        assert_eq!(unsupported_example.is_https, false);
        assert_eq!(unsupported_example.is_http, false);
        assert_eq!(unsupported_example.is_supported, false);

        let first_party = build_request(
            "document",
            "https://subdomain.example.com/ad",
            "https",
            "subdomain.example.com",
            "example.com",
            "example.com",
            "example.com",
        );
        assert_eq!(first_party.is_https, true);
        assert_eq!(first_party.is_supported, true);
        assert_eq!(first_party.is_third_party, false);

        let third_party = build_request(
            "document",
            "https://subdomain.anotherexample.com/ad",
            "https",
            "subdomain.anotherexample.com",
            "anotherexample.com",
            "example.com",
            "example.com",
        );
        assert_eq!(third_party.is_https, true);
        assert_eq!(third_party.is_supported, true);
        assert_eq!(third_party.is_third_party, true);

        let websocket = build_request(
            "document",
            "wss://subdomain.anotherexample.com/ad",
            "wss",
            "subdomain.anotherexample.com",
            "anotherexample.com",
            "example.com",
            "example.com",
        );
        assert_eq!(websocket.is_https, false);
        assert_eq!(websocket.is_https, false);
        assert_eq!(websocket.is_supported, true);
        assert_eq!(websocket.is_third_party, true);
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
        assert_eq!(assumed_https.is_https, true);
        assert_eq!(assumed_https.is_http, false);
        assert_eq!(assumed_https.is_supported, true);
    }

    fn tokenize(tokens: &[&str], extra_tokens: &[utils::Hash]) -> Vec<utils::Hash> {
        let mut tokens: Vec<_> = tokens.into_iter().map(|t| utils::fast_hash(&t)).collect();
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
        assert_eq!(parsed.is_https, true);
        assert_eq!(parsed.is_supported, true);
        assert_eq!(parsed.is_third_party, false);
        assert_eq!(parsed.request_type, RequestType::Document);

        // assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.hostname, "subdomain.example.com");

        // assert_eq!(parsed.source_domain, "example.com");
        assert_eq!(
            parsed.source_hostname_hashes.unwrap().as_slice(),
            [utils::fast_hash("example.com"), utils::fast_hash("com")],
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
