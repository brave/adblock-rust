
use crate::utils;

use addr::{DomainName};
use url::{Url};
use std::collections::HashMap;

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
    SourceHostnameParseError
}

lazy_static! {
    static ref CPT_TO_TYPE: HashMap<&'static str, RequestType> = {
        let mut map = HashMap::new();
        map.insert("beacon" , RequestType::Ping);
        map.insert("csp_report" , RequestType::Csp);
        map.insert("document" , RequestType::Document);
        map.insert("font" , RequestType::Font);
        map.insert("image" , RequestType::Image);
        map.insert("imageset" , RequestType::Image);
        map.insert("main_frame" , RequestType::Document);
        map.insert("media" , RequestType::Media);
        map.insert("object" , RequestType::Object);
        map.insert("object_subrequest" , RequestType::Object);
        map.insert("other" , RequestType::Other);
        map.insert("ping" , RequestType::Ping);
        map.insert("script" , RequestType::Script);
        map.insert("speculative" , RequestType::Other);
        map.insert("stylesheet" , RequestType::Stylesheet);
        map.insert("sub_frame" , RequestType::Subdocument);
        map.insert("web_manifest" , RequestType::Other);
        map.insert("websocket" , RequestType::Websocket);
        map.insert("xbl" , RequestType::Other);
        map.insert("xhr" , RequestType::Xmlhttprequest);
        map.insert("xml_dtd" , RequestType::Other);
        map.insert("xmlhttprequest" , RequestType::Xmlhttprequest);
        map.insert("xslt" , RequestType::Other);
        map
    };
}

pub struct Request {
    pub raw_type: String,
    pub request_type: RequestType,
    
    pub is_http: bool,
    pub is_https: bool,
    pub is_supported: bool,
    pub is_first_party: Option<bool>,
    pub is_third_party: Option<bool>,
    pub url: String,
    pub hostname: String,
    pub domain: String,
    pub source_url: String,
    pub source_hostname: String,
    pub source_hostname_hash: utils::Hash,
    pub source_domain: String,
    pub source_domain_hash: utils::Hash,

    // mutable fields, set later
    pub bug: Option<u32>,
    tokens: Option<Vec<utils::Hash>>, // evaluated lazily
    fuzzy_signature: Option<Vec<utils::Hash>> // evaluated lazily
}

impl<'a> Request {
    pub fn new(
        raw_type: &str,
        url: &str,
        hostname: &str,
        domain: &str,
        source_url: &str,
        source_hostname: &str,
        source_domain: &str,
    ) -> Request {

        let first_party = if source_domain.len() == 0 { None } else { Some(source_domain == domain) };
        let third_party = first_party.map(|p| !p);

        let mut splitter = url.splitn(2, ':');
        let protocol: &str = splitter.next().unwrap();
        let remainder: Option<&str> = splitter.next();
        let is_http: bool;
        let is_https: bool;
        let is_supported: bool;
        let mut request_type: RequestType = CPT_TO_TYPE.get(&raw_type).map(|v| v.to_owned()).unwrap_or(RequestType::Other);
        if remainder.is_none() { // no ':' was found
            is_https = true;
            is_http = false;
            is_supported = true;
        } else {
            is_http = protocol == "http";
            is_https = protocol == "https";

            let is_websocket = !is_http && !is_https && (protocol == "ws" || protocol == "wss");
            is_supported = is_http || is_https || is_websocket;
            if is_websocket {
                request_type = RequestType::Websocket;
            }
        }

        Request {
            raw_type: String::from(raw_type),
            request_type: request_type,
            url: String::from(url),
            hostname: String::from(hostname),
            domain: String::from(domain),
            source_url: String::from(source_url),
            source_hostname: String::from(source_hostname),
            source_domain: String::from(source_domain),
            source_hostname_hash: utils::fast_hash(&source_hostname),
            source_domain_hash: utils::fast_hash(&source_domain),
            is_first_party: first_party,
            is_third_party: third_party,
            is_http: is_http,
            is_https: is_https,
            is_supported: is_supported,
            bug: None,
            tokens: None,
            fuzzy_signature: None
        }
    }

    pub fn get_tokens(&mut self) -> &Vec<utils::Hash> {
        if self.tokens.is_none() {
            let mut tokens_buffer: [utils::Hash; 2] = [0; 2];
            let mut tokens_buffer_index: usize = 0;
            tokens_buffer[tokens_buffer_index] = utils::fast_hash(&self.source_domain);
            tokens_buffer_index = tokens_buffer_index + 1;
            tokens_buffer[tokens_buffer_index] = utils::fast_hash(&self.source_hostname);
            tokens_buffer_index = tokens_buffer_index + 1;

            let mut url_tokens = utils::tokenize(&self.url);
            let mut tokens = tokens_buffer[0..tokens_buffer_index].to_vec();
            tokens.append(&mut url_tokens);

            self.tokens = Some(tokens);
        }
        &self.tokens.as_ref().unwrap()
    }

    pub fn get_fuzzy_signature(&mut self) -> &Vec<utils::Hash> {
        if self.fuzzy_signature.is_none() {
            self.fuzzy_signature = Some(utils::create_fuzzy_signature(&self.url));
        }
        &self.fuzzy_signature.as_ref().unwrap()
    }


    pub fn from_urls(url: &str, source_url: &str, request_type: &str) -> Result<Request, RequestError> {
        let url_norm = url.to_lowercase();
        let maybe_hostname = get_url_host(&url_norm);
        if maybe_hostname.is_none() {
            return Err(RequestError::HostnameParseError);
        }
        let hostname = maybe_hostname.unwrap();
        let domain = get_host_domain(&hostname);

        let source_url_norm = source_url.to_lowercase();
        let maybe_source_hostname = get_url_host(&source_url_norm);
        if maybe_source_hostname.is_none() {
            return Err(RequestError::SourceHostnameParseError);
        }
        let source_hostname = maybe_source_hostname.unwrap();
        let source_domain = get_host_domain(&source_hostname);

        Ok(Request::new(request_type,
            &url_norm,
            &hostname,
            &domain,
            &source_url_norm,
            &source_hostname,
            &source_domain))
    }

    pub fn from_urls_with_hostname(url: &str, hostname: &str, source_url: &str, source_hostname: &str, request_type: &str) -> Request {
        let url_norm = url.to_lowercase();
        let domain = get_host_domain(&hostname);

        let source_url_norm = source_url.to_lowercase();
        let source_domain = get_host_domain(&source_hostname);

        Request::new(request_type, &url_norm, &hostname, &domain, &source_url_norm, &source_hostname, &source_domain)
    }
}

pub fn get_host_domain(host: &str) -> String {
    match host.parse::<DomainName>() {
        Err(_e) => String::from(host),
        Ok(domain) => String::from(domain.root().to_str())
    }
}

pub fn get_url_host(url: &str) -> Option<String> {
  url.parse::<Url>()
    .ok() // convert to Option
    .and_then(|p| p.host_str().map(String::from))
    
}

pub fn get_url_domain(url: &str) -> Option<String> {
    get_url_host(&url).map(|host| get_host_domain(&host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_works() {
        let simple_example = Request::new("document", "https://example.com/ad", "example.com", "example.com", "https://example.com", "example.com", "example.com");
        assert_eq!(simple_example.is_https, true);
        assert_eq!(simple_example.is_supported, true);
        assert_eq!(simple_example.is_first_party, Some(true));
        assert_eq!(simple_example.is_third_party, Some(false));
        assert_eq!(simple_example.request_type, RequestType::Document);
        assert_eq!(simple_example.source_domain_hash, utils::fast_hash("example.com"));
        assert_eq!(simple_example.source_hostname_hash, utils::fast_hash("example.com"));

        let unsupported_example = Request::new("document", "file://example.com/ad", "example.com", "example.com", "https://example.com", "example.com", "example.com");
        assert_eq!(unsupported_example.is_https, false);
        assert_eq!(unsupported_example.is_http, false);
        assert_eq!(unsupported_example.is_supported, false);

        let first_party = Request::new("document", "https://subdomain.example.com/ad", "subdomain.example.com", "example.com", "https://example.com", "example.com", "example.com");
        assert_eq!(first_party.is_https, true);
        assert_eq!(first_party.is_supported, true);
        assert_eq!(first_party.is_first_party, Some(true));
        assert_eq!(first_party.is_third_party, Some(false));
        
        let third_party = Request::new("document", "https://subdomain.anotherexample.com/ad", "subdomain.anotherexample.com", "anotherexample.com", "https://example.com", "example.com", "example.com");
        assert_eq!(third_party.is_https, true);
        assert_eq!(third_party.is_supported, true);
        assert_eq!(third_party.is_first_party, Some(false));
        assert_eq!(third_party.is_third_party, Some(true));

        let websocket = Request::new("document", "wss://subdomain.anotherexample.com/ad", "subdomain.anotherexample.com", "anotherexample.com", "https://example.com", "example.com", "example.com");
        assert_eq!(websocket.is_https, false);
        assert_eq!(websocket.is_https, false);
        assert_eq!(websocket.is_supported, true);
        assert_eq!(websocket.is_first_party, Some(false));
        assert_eq!(websocket.is_third_party, Some(true));
        assert_eq!(websocket.request_type, RequestType::Websocket);

        let assumed_https = Request::new("document", "//subdomain.anotherexample.com/ad", "subdomain.anotherexample.com", "anotherexample.com", "https://example.com", "example.com", "example.com");
        assert_eq!(assumed_https.is_https, true);
        assert_eq!(assumed_https.is_http, false);
        assert_eq!(assumed_https.is_supported, true);
    }

    fn t(tokens: &[&str]) -> Vec<utils::Hash> {
        tokens.into_iter().map(|t| utils::fast_hash(&t)).collect()
    }

    #[test]
    fn get_fuzzy_signature_works() {
        let mut simple_example = Request::new("document", "https://example.com/ad", "example.com", "example.com", "https://example.com", "example.com", "example.com");
        assert_eq!(simple_example.get_fuzzy_signature().as_slice(), t(&vec!["ad", "https", "com", "example"]).as_slice())
    }

    #[test]
    fn tokens_works() {
        let mut simple_example = Request::new("document", "https://subdomain.example.com/ad", "subdomain.example.com", "example.com", "https://subdomain.example.com", "subdomain.example.com", "example.com");
        assert_eq!(simple_example.get_tokens().as_slice(), t(&vec!["example.com", "subdomain.example.com", "https", "subdomain", "example", "com", "ad"]).as_slice())
    }

    #[test]
    fn parses_urls() {
        let parsed = Request::from_urls("https://subdomain.example.com/ad", "https://example.com/", "document").unwrap();
        assert_eq!(parsed.is_https, true);
        assert_eq!(parsed.is_supported, true);
        assert_eq!(parsed.is_first_party, Some(true));
        assert_eq!(parsed.is_third_party, Some(false));
        assert_eq!(parsed.request_type, RequestType::Document);

        assert_eq!(parsed.domain, "example.com");
        assert_eq!(parsed.hostname, "subdomain.example.com");

        assert_eq!(parsed.source_domain, "example.com");
        assert_eq!(parsed.source_domain_hash, utils::fast_hash("example.com"));
        assert_eq!(parsed.source_hostname, "example.com");
        assert_eq!(parsed.source_hostname_hash, utils::fast_hash("example.com"));

        let bad_url = Request::from_urls("subdomain.example.com/ad", "https://example.com/", "document");
        assert_eq!(bad_url.err(), Some(RequestError::HostnameParseError));

        let bad_source_url = Request::from_urls("https://subdomain.example.com/ad", "example.com/", "document");
        assert_eq!(bad_source_url.err(), Some(RequestError::SourceHostnameParseError));
    }
}