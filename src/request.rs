use crate::url_parser::{get_host_domain, UrlParser};
use crate::utils;

use idna;
use std::collections::HashMap;
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

impl From<idna::uts46::Errors> for RequestError {
    fn from(_err: idna::uts46::Errors) -> RequestError {
        RequestError::UnicodeDecodingError
    }
}

impl From<url::ParseError> for RequestError {
    fn from(_err: url::ParseError) -> RequestError {
        RequestError::HostnameParseError
    }
}

lazy_static! {
    static ref CPT_TO_TYPE: HashMap<&'static str, RequestType> = {
        let mut map = HashMap::new();
        map.insert("beacon", RequestType::Ping);
        map.insert("csp_report", RequestType::Csp);
        map.insert("document", RequestType::Document);
        map.insert("font", RequestType::Font);
        map.insert("image", RequestType::Image);
        map.insert("imageset", RequestType::Image);
        map.insert("main_frame", RequestType::Document);
        map.insert("media", RequestType::Media);
        map.insert("object", RequestType::Object);
        map.insert("object_subrequest", RequestType::Object);
        map.insert("other", RequestType::Other);
        map.insert("ping", RequestType::Ping);
        map.insert("script", RequestType::Script);
        map.insert("speculative", RequestType::Other);
        map.insert("stylesheet", RequestType::Stylesheet);
        map.insert("sub_frame", RequestType::Subdocument);
        map.insert("web_manifest", RequestType::Other);
        map.insert("websocket", RequestType::Websocket);
        map.insert("xbl", RequestType::Other);
        map.insert("xhr", RequestType::Xmlhttprequest);
        map.insert("xml_dtd", RequestType::Other);
        map.insert("xmlhttprequest", RequestType::Xmlhttprequest);
        map.insert("xslt", RequestType::Other);
        map
    };
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
    #[cfg(feature = "full-domain-matching")]
    pub source_hostname: String,
    #[cfg(feature = "full-domain-matching")]
    pub source_domain: String,
    pub source_hostname_hashes: Option<Vec<utils::Hash>>,

    // mutable fields, set later
    pub bug: Option<u32>,
    tokens: Arc<RwLock<Option<Vec<utils::Hash>>>>, // evaluated lazily
    fuzzy_signature: Arc<RwLock<Option<Vec<utils::Hash>>>>, // evaluated lazily
}

impl<'a> Request {
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

        Self::from_detailed_parameters(
            raw_type,
            url,
            schema,
            hostname,
            source_hostname,
            source_domain,
            third_party,
        )
    }

    fn from_detailed_parameters(
        raw_type: &str,
        url: &str,
        schema: &str,
        hostname: &str,
        source_hostname: &str,
        source_domain: &str,
        third_party: Option<bool>,
    ) -> Request {
        let first_party = third_party.map(|p| !p);

        let is_http: bool;
        let is_https: bool;
        let is_supported: bool;
        let mut request_type: RequestType = CPT_TO_TYPE
            .get(&raw_type)
            .map(|v| v.to_owned())
            .unwrap_or(RequestType::Other);

        if schema.is_empty() {
            // no ':' was found
            is_https = true;
            is_http = false;
            is_supported = true;
        } else {
            is_http = schema == "http";
            is_https = schema == "https";

            let is_websocket = !is_http && !is_https && (schema == "ws" || schema == "wss");
            is_supported = is_http || is_https || is_websocket;
            if is_websocket {
                request_type = RequestType::Websocket;
            }
        }

        let source_hostname_hashes = if !source_hostname.is_empty() {
            // println!("Processing URL {}", url);
            let mut hashes = Vec::with_capacity(2);
            hashes.push(utils::fast_hash(&source_hostname));
            for (i, c) in
                source_hostname[..source_hostname.len() - source_domain.len()].char_indices()
            {
                if c == '.' {
                    // println!("Hashing hostname part {} of {}", &source_hostname[i+1..], url);
                    hashes.push(utils::fast_hash(&source_hostname[i + 1..]));
                }
            }
            hashes.shrink_to_fit();
            Some(hashes)
        } else {
            None
        };

        Request {
            request_type,
            url: String::from(url),
            hostname: String::from(hostname),
            #[cfg(feature = "full-domain-matching")]
            source_hostname: String::from(source_hostname),
            #[cfg(feature = "full-domain-matching")]
            source_domain: String::from(source_domain),
            source_hostname_hashes,
            is_first_party: first_party,
            is_third_party: third_party,
            is_http,
            is_https,
            is_supported,
            bug: None,
            tokens: Arc::new(RwLock::new(None)),
            fuzzy_signature: Arc::new(RwLock::new(None)),
        }
    }

    pub fn get_tokens(&self) -> Vec<utils::Hash> {
        // Create a new scope to contain the lifetime of the
        // dynamic borrow
        {
            let tokens_cache = self.tokens.read().unwrap();
            if tokens_cache.is_some() {
                return tokens_cache.as_ref().unwrap().clone();
            }
        }
        {
            let mut tokens_cache = self.tokens.write().unwrap();
            let mut tokens: Vec<utils::Hash> = vec![];

            if let Some(hashes) = self.source_hostname_hashes.as_ref() {
                for h in hashes {
                    tokens.push(*h)
                }
            }

            let mut url_tokens = utils::tokenize(&self.url);
            tokens.append(&mut url_tokens);

            *tokens_cache = Some(tokens);
        }
        // Recursive call to return the just-cached value.
        // Note that if we had not let the previous borrow
        // of the cache fall out of scope then the subsequent
        // recursive borrow would cause a dynamic thread panic.
        // This is the major hazard of using `RefCell`.
        self.get_tokens()
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

    pub fn from_urls(
        url: &str,
        source_url: &str,
        request_type: &str,
    ) -> Result<Request, RequestError> {
        let url_norm = url.to_ascii_lowercase();
        let source_url_norm = source_url.to_ascii_lowercase();

        let maybe_parsed_url = Request::get_url_host(&url_norm);
        if maybe_parsed_url.is_none() {
            return Err(RequestError::HostnameParseError);
        }
        let parsed_url = maybe_parsed_url.unwrap();

        let maybe_parsed_source = Request::get_url_host(&source_url_norm);

        if maybe_parsed_source.is_none() {
            Ok(Request::new(
                request_type,
                &parsed_url.url,
                parsed_url.schema(),
                parsed_url.hostname(),
                &parsed_url.domain,
                "",
                "",
            ))
        } else {
            let parsed_source = maybe_parsed_source.unwrap();
            Ok(Request::new(
                request_type,
                &parsed_url.url,
                parsed_url.schema(),
                parsed_url.hostname(),
                &parsed_url.domain,
                parsed_source.hostname(),
                &parsed_source.domain,
            ))
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

        let source_domain = get_host_domain(&source_hostname);

        let splitter = url_norm.find(':').unwrap_or(0);
        let protocol: &str = &url[..splitter];

        let third_party = if third_party_request.is_none() {
            let domain = get_host_domain(&hostname);
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
            &protocol,
            &hostname,
            &source_hostname,
            &source_domain,
            third_party
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

    fn t(tokens: &[&str]) -> Vec<utils::Hash> {
        tokens.into_iter().map(|t| utils::fast_hash(&t)).collect()
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
        let mut tokens = t(&vec!["ad", "https", "com", "example"]);
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
            simple_example.get_tokens().as_slice(),
            t(&vec![
                "subdomain.example.com",
                "example.com",
                "https",
                "subdomain",
                "example",
                "com",
                "ad"
            ])
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

        // let bad_source_url = Request::from_urls("https://subdomain.example.com/ad", "example.com/", "document");
        // assert_eq!(bad_source_url.err(), Some(RequestError::SourceHostnameParseError));
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
}
