use crate::url_parser::{get_host_domain, UrlParser};
use crate::utils;

use idna;
use std::sync::{Arc, RwLock};

#[derive(Clone, PartialEq, Debug)]
pub enum RequestType {
    Beacon,
    Csp,
    Document,
    Dtd,
    Fetch,
    Font,
    Image,
    Media,
    Object,
    Other,
    Ping,
    Script,
    Stylesheet,
    Subdocument,
    Websocket,
    Xlst,
    Xmlhttprequest,
}

#[derive(Debug, PartialEq)]
pub enum RequestError {
    HostnameParseError,
    SourceHostnameParseError,
    UnicodeDecodingError,
}

impl From<idna::Errors> for RequestError {
    fn from(_err: idna::Errors) -> RequestError {
        RequestError::UnicodeDecodingError
    }
}

impl From<url::ParseError> for RequestError {
    fn from(_err: url::ParseError) -> RequestError {
        RequestError::HostnameParseError
    }
}

fn cpt_match_type(cpt: &str) -> RequestType {
    match cpt {
        "beacon" => RequestType::Ping,
        "csp_report" => RequestType::Csp,
        "document" | "main_frame" => RequestType::Document,
        "font" => RequestType::Font,
        "image" | "imageset" => RequestType::Image,
        "media" => RequestType::Media,
        "object" | "object_subrequest" => RequestType::Object,
        "ping" => RequestType::Ping,
        "script" => RequestType::Script,
        "stylesheet" => RequestType::Stylesheet,
        "sub_frame" | "subdocument" => RequestType::Subdocument,
        "websocket" => RequestType::Websocket,
        "xhr" | "xmlhttprequest" => RequestType::Xmlhttprequest,
        "other" => RequestType::Other,
        "speculative" => RequestType::Other,
        "web_manifest" => RequestType::Other,
        "xbl" => RequestType::Other,
        "xml_dtd" => RequestType::Other,
        "xslt" => RequestType::Other,
        _ => RequestType::Other
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub request_type: RequestType,

    pub is_http: bool,
    pub is_https: bool,
    pub is_supported: bool,
    pub is_first_party: Option<bool>,
    pub is_third_party: Option<bool>,
    pub url: String,
    pub hostname: String,
    pub source_hostname_hashes: Option<Vec<utils::Hash>>,

    // mutable fields, set later
    pub bug: Option<u32>,
    fuzzy_signature: Arc<RwLock<Option<Vec<utils::Hash>>>>, // evaluated lazily
    hostname_end: usize
}

impl<'a> Request {
    pub fn get_tokens(&self, mut token_buffer: &mut Vec<utils::Hash>) {
        token_buffer.clear();
        utils::tokenize_pooled(&self.url, &mut token_buffer);
        // Add zero token as a fallback to wildcard rule bucket
        token_buffer.push(0);
    }

    pub fn url_after_hostname(&self) -> &str {
        &self.url[self.hostname_end..]
    }

    pub fn get_fuzzy_signature(&self) -> Vec<utils::Hash> {
        {
            let signature_cache = self.fuzzy_signature.read().unwrap();
            if signature_cache.is_some() {
                return signature_cache.as_ref().unwrap().clone();
            }
        }
        {
            let mut signature_cache = self.fuzzy_signature.write().unwrap();
            let signature = utils::create_fuzzy_signature(&self.url);
            *signature_cache = Some(signature);
        }
        self.get_fuzzy_signature()
    }

    pub fn new(
        raw_type: &str,
        url: &str,
        schema: &str,
        hostname: &str,
        domain: &str,
        source_hostname: &str,
        source_domain: &str,
    ) -> Request {
        let third_party = if source_domain.is_empty() {
            None
        } else {
            Some(source_domain != domain)
        };

        let hostname_end = twoway::find_str(url, hostname).unwrap_or_else(|| url.len()) + hostname.len();

        Self::from_detailed_parameters(
            raw_type,
            url,
            schema,
            hostname,
            source_hostname,
            source_domain,
            third_party,
            hostname_end
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_detailed_parameters(
        raw_type: &str,
        url: &str,
        schema: &str,
        hostname: &str,
        source_hostname: &str,
        source_domain: &str,
        third_party: Option<bool>,
        hostname_end: usize
    ) -> Request {
        let first_party = third_party.map(|p| !p);

        let is_http: bool;
        let is_https: bool;
        let is_supported: bool;
        let request_type: RequestType;

        if schema.is_empty() {
            // no ':' was found
            is_https = true;
            is_http = false;
            is_supported = true;
            request_type = cpt_match_type(raw_type);
        } else {
            is_http = schema == "http";
            is_https = !is_http && schema == "https";

            let is_websocket = !is_http && !is_https && (schema == "ws" || schema == "wss");
            is_supported = is_http || is_https || is_websocket;
            if is_websocket {
                request_type = RequestType::Websocket;
            } else {
                request_type = cpt_match_type(raw_type);
            }
        }

        let source_hostname_hashes = if !source_hostname.is_empty() {
            let mut hashes = Vec::with_capacity(4);
            hashes.push(utils::fast_hash(&source_hostname));
            for (i, c) in
                source_hostname[..source_hostname.len() - source_domain.len()].char_indices()
            {
                if c == '.' {
                    hashes.push(utils::fast_hash(&source_hostname[i + 1..]));
                }
            }
            Some(hashes)
        } else {
            None
        };

        Request {
            request_type,
            url: url.to_owned(),
            hostname: hostname.to_owned(),
            source_hostname_hashes,
            is_first_party: first_party,
            is_third_party: third_party,
            is_http,
            is_https,
            is_supported,
            bug: None,
            fuzzy_signature: Arc::new(RwLock::new(None)),
            hostname_end
        }
    }

    pub fn from_urls(
        url: &str,
        source_url: &str,
        request_type: &str,
    ) -> Result<Request, RequestError> {
        if let Some(parsed_url) = Request::parse_url(&url) {
            if let Some(parsed_source) = Request::parse_url(&source_url) {
                let source_domain = parsed_source.domain();

                let third_party = if source_domain.is_empty() {
                    None
                } else {
                    Some(source_domain != parsed_url.domain())
                };

                Ok(Request::from_detailed_parameters(
                    request_type,
                    &parsed_url.url,
                    parsed_url.schema(),
                    parsed_url.hostname(),
                    parsed_source.hostname(),
                    source_domain,
                    third_party,
                    parsed_url.hostname_pos.1
                ))
            } else {
                Ok(Request::from_detailed_parameters(
                    request_type,
                    &parsed_url.url,
                    parsed_url.schema(),
                    parsed_url.hostname(),
                    "",
                    "",
                    None,
                    parsed_url.hostname_pos.1
                ))
            }
        } else {
            return Err(RequestError::HostnameParseError);
        }
    }

    pub fn from_urls_with_hostname(
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party_request: Option<bool>
    ) -> Request {
        let url_norm = url.to_ascii_lowercase();

        let (source_domain_start, source_domain_end) = get_host_domain(&source_hostname);
        let source_domain = &source_hostname[source_domain_start..source_domain_end];

        let splitter = url_norm.find(':').unwrap_or(0);
        let schema: &str = &url[..splitter];

        let third_party = if third_party_request.is_none() {
            let (domain_start, domain_end) = get_host_domain(&hostname);
            let domain = &hostname[domain_start..domain_end];
            if source_domain.is_empty() {
                None
            } else {
                Some(source_domain != domain)
            }
        } else {
            third_party_request
        };

        Request::from_detailed_parameters(
            request_type,
            &url_norm,
            &schema,
            &hostname,
            &source_hostname,
            &source_domain,
            third_party,
            splitter + 2 + hostname.len()
        )
    }

    pub fn from_url(url: &str) -> Result<Request, RequestError> {
        // Used in testing - assume empty source_url and default request type
        Self::from_urls(url, "", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_works() {
        let simple_example = Request::new(
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
        assert_eq!(simple_example.is_first_party, Some(true));
        assert_eq!(simple_example.is_third_party, Some(false));
        assert_eq!(simple_example.request_type, RequestType::Document);
        assert_eq!(
            simple_example
                .source_hostname_hashes
                .as_ref()
                .and_then(|h| h.last()),
            Some(&utils::fast_hash("example.com"))
        );
        assert_eq!(
            simple_example
                .source_hostname_hashes
                .as_ref()
                .and_then(|h| h.first()),
            Some(&utils::fast_hash("example.com"))
        );

        let unsupported_example = Request::new(
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

        let first_party = Request::new(
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
        assert_eq!(first_party.is_first_party, Some(true));
        assert_eq!(first_party.is_third_party, Some(false));

        let third_party = Request::new(
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
        assert_eq!(third_party.is_first_party, Some(false));
        assert_eq!(third_party.is_third_party, Some(true));

        let websocket = Request::new(
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
        assert_eq!(websocket.is_first_party, Some(false));
        assert_eq!(websocket.is_third_party, Some(true));
        assert_eq!(websocket.request_type, RequestType::Websocket);

        let assumed_https = Request::new(
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
    fn get_fuzzy_signature_works() {
        let simple_example = Request::new(
            "document",
            "https://example.com/ad",
            "https",
            "example.com",
            "example.com",
            "example.com",
            "example.com",
        );
        let mut tokens = tokenize(&["ad", "https", "com", "example"], &[]);
        tokens.sort_unstable();
        assert_eq!(
            simple_example.get_fuzzy_signature().as_slice(),
            tokens.as_slice()
        )
    }

    #[test]
    fn tokens_works() {
        let simple_example = Request::new(
            "document",
            "https://subdomain.example.com/ad",
            "https",
            "subdomain.example.com",
            "example.com",
            "subdomain.example.com",
            "example.com",
        );
        assert_eq!(
            simple_example.source_hostname_hashes.as_ref().unwrap().as_slice(),
            tokenize(&[
                "subdomain.example.com",
                "example.com",
            ], &[])
            .as_slice()
        );
        let mut tokens = Vec::new();
        simple_example.get_tokens(&mut tokens);
        assert_eq!(
            tokens.as_slice(),
            tokenize(&[
                "https",
                "subdomain",
                "example",
                "com",
                "ad"
            ], &[0])
            .as_slice()
        )
    }

    #[test]
    fn parses_urls() {
        let parsed = Request::from_urls(
            "https://subdomain.example.com/ad",
            "https://example.com/",
            "document",
        )
        .unwrap();
        assert_eq!(parsed.is_https, true);
        assert_eq!(parsed.is_supported, true);
        assert_eq!(parsed.is_first_party, Some(true));
        assert_eq!(parsed.is_third_party, Some(false));
        assert_eq!(parsed.request_type, RequestType::Document);

        // assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.hostname, "subdomain.example.com");

        // assert_eq!(parsed.source_domain, "example.com");
        assert_eq!(
            parsed
                .source_hostname_hashes
                .as_ref()
                .and_then(|h| h.first()),
            Some(&utils::fast_hash("example.com"))
        );
        // assert_eq!(parsed.source_hostname, "example.com");
        assert_eq!(
            parsed
                .source_hostname_hashes
                .as_ref()
                .and_then(|h| h.last()),
            Some(&utils::fast_hash("example.com"))
        );

        let bad_url = Request::from_urls(
            "subdomain.example.com/ad",
            "https://example.com/",
            "document",
        );
        assert_eq!(bad_url.err(), Some(RequestError::HostnameParseError));
    }

    #[test]
    fn handles_explicit_third_party_param() {
        {
            // domain matches
            let parsed = Request::from_urls_with_hostname("https://subdomain.example.com/ad", "subdomain.example.com", "example.com", "document", None);
            assert_eq!(parsed.is_third_party, Some(false));
        }
        {
            // domain does not match
            let parsed = Request::from_urls_with_hostname("https://subdomain.example.com/ad", "subdomain.example.com", "anotherexample.com", "document", None);
            assert_eq!(parsed.is_third_party, Some(true));
        }
        {
            // cannot parse domain
            let parsed = Request::from_urls_with_hostname("https://subdomain.example.com/ad", "subdomain.example.com", "", "document", None);
            assert_eq!(parsed.is_third_party, None);
        }
        {
            // third-partiness set to false
            let parsed = Request::from_urls_with_hostname("https://subdomain.example.com/ad", "subdomain.example.com", "example.com", "document", Some(true));
            assert_eq!(parsed.is_third_party, Some(true));
        }
        {
            // third-partiness set to true
            let parsed = Request::from_urls_with_hostname("https://subdomain.example.com/ad", "subdomain.example.com", "anotherexample.com", "document", Some(false));
            assert_eq!(parsed.is_third_party, Some(false));
        }
    }

    #[test]
    fn fuzzing_errors() {
        {
            let parsed = Request::from_url("https://߶");
            assert!(parsed.is_ok());
        }
        {
            let parsed = Request::from_url(
                &format!("https://{}", std::str::from_utf8(&[9, 9, 64]).unwrap()));
            assert!(parsed.is_err());
        }
    }
}
