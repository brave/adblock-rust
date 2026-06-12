//! Contains structures needed to describe network requests.

use std::sync::OnceLock;

use thiserror::Error;

use crate::filters::cosmetic::get_entity_hashes_from_labels;
use crate::url_parser;
use crate::utils::{self, HostnameHashBuffer};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RequestMethod {
    Connect,
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
    Other,
}

/// The type of resource requested from the URL endpoint.
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

/// Possible failure reasons when creating a [`Request`].
#[derive(Debug, Error, PartialEq)]
pub enum RequestError {
    #[error("hostname parsing failed")]
    HostnameParseError,
    #[error("source hostname parsing failed")]
    SourceHostnameParseError,
    #[error("invalid Unicode provided")]
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
        _ => RequestType::Other,
    }
}

/// A network [`Request`], used as an interface for network blocking in the [`crate::Engine`].
#[derive(Debug)]
pub struct Request {
    pub request_type: RequestType,
    pub method: Option<RequestMethod>,

    pub is_http: bool,
    pub is_https: bool,
    pub is_supported: bool,
    pub is_third_party: bool,
    pub url: String,
    pub hostname: String,
    pub(crate) domain: String,
    pub(crate) source_hostname: String,
    pub source_hostname_hashes: Option<Vec<utils::Hash>>,

    destination_suffix_hashes: OnceLock<HostnameHashBuffer>,
    destination_entity_hashes: OnceLock<HostnameHashBuffer>,

    pub(crate) url_lower_cased: String,
    pub(crate) request_tokens: Vec<utils::Hash>,
    pub(crate) original_url: String,
}

impl Clone for Request {
    fn clone(&self) -> Self {
        Self {
            request_type: self.request_type.clone(),
            method: self.method,
            is_http: self.is_http,
            is_https: self.is_https,
            is_supported: self.is_supported,
            is_third_party: self.is_third_party,
            url: self.url.clone(),
            hostname: self.hostname.clone(),
            domain: self.domain.clone(),
            source_hostname: self.source_hostname.clone(),
            source_hostname_hashes: self.source_hostname_hashes.clone(),
            destination_suffix_hashes: clone_once_lock(&self.destination_suffix_hashes),
            destination_entity_hashes: clone_once_lock(&self.destination_entity_hashes),
            url_lower_cased: self.url_lower_cased.clone(),
            request_tokens: self.request_tokens.clone(),
            original_url: self.original_url.clone(),
        }
    }
}

fn clone_once_lock(src: &OnceLock<HostnameHashBuffer>) -> OnceLock<HostnameHashBuffer> {
    let lock = OnceLock::new();
    if let Some(buffer) = src.get() {
        let _ = lock.set(buffer.clone());
    }
    lock
}

impl Request {
    pub(crate) fn get_url(&self, case_sensitive: bool) -> &str {
        if case_sensitive {
            &self.url
        } else {
            &self.url_lower_cased
        }
    }

    pub fn get_tokens_for_match(&self) -> impl Iterator<Item = &utils::Hash> {
        // We start matching with source_hostname_hashes for optimization,
        // as it contains far fewer elements.
        self.source_hostname_hashes
            .as_ref()
            .into_iter()
            .flatten()
            .chain(self.get_tokens())
    }

    pub fn get_tokens(&self) -> &Vec<utils::Hash> {
        &self.request_tokens
    }

    /// Lazily computed destination suffix hashes for `$to=` plain matching.
    pub(crate) fn destination_suffix_hashes(&self) -> Option<&[utils::Hash]> {
        if self.hostname.is_empty() {
            return None;
        }
        Some(
            self.destination_suffix_hashes
                .get_or_init(|| self.compute_destination_suffix_hashes())
                .as_slice(),
        )
    }

    /// Lazily computed destination entity hashes for `$to=google.*` style matching.
    pub(crate) fn destination_entity_hashes(&self) -> Option<&[utils::Hash]> {
        if self.hostname.is_empty() {
            return None;
        }
        let buffer = self.destination_entity_hashes.get_or_init(|| {
            let mut buffer = HostnameHashBuffer::new();
            for hash in get_entity_hashes_from_labels(&self.hostname, &self.domain) {
                if buffer.try_push(hash).is_err() {
                    break;
                }
            }
            buffer
        });
        if buffer.is_empty() {
            None
        } else {
            Some(buffer.as_slice())
        }
    }

    fn compute_destination_suffix_hashes(&self) -> HostnameHashBuffer {
        let mut buffer = HostnameHashBuffer::new();
        if self.hostname.is_empty() {
            return buffer;
        }
        if self.hostname == self.source_hostname {
            if let Some(source_hashes) = self.source_hostname_hashes.as_ref() {
                for &hash in source_hashes {
                    if buffer.try_push(hash).is_err() {
                        break;
                    }
                }
                return buffer;
            }
        }
        Self::push_suffix_hostname_hashes(&self.hostname, &mut buffer);
        buffer
    }

    fn push_suffix_hostname_hashes(hostname: &str, buffer: &mut HostnameHashBuffer) {
        let _ = buffer.try_push(utils::fast_hash(hostname));
        for (i, c) in hostname.char_indices() {
            if c == '.' && i + 1 < hostname.len() {
                let _ = buffer.try_push(utils::fast_hash(&hostname[i + 1..]));
            }
        }
    }

    fn suffix_hostname_hashes(hostname: &str) -> Option<Vec<utils::Hash>> {
        if hostname.is_empty() {
            return None;
        }
        let mut buffer = HostnameHashBuffer::new();
        Self::push_suffix_hostname_hashes(hostname, &mut buffer);
        Some(buffer.into_iter().collect())
    }

    fn parse_method(raw_method: &str) -> Option<RequestMethod> {
        if raw_method.is_empty() {
            return None;
        }
        Some(if raw_method.eq_ignore_ascii_case("connect") {
            RequestMethod::Connect
        } else if raw_method.eq_ignore_ascii_case("delete") {
            RequestMethod::Delete
        } else if raw_method.eq_ignore_ascii_case("get") {
            RequestMethod::Get
        } else if raw_method.eq_ignore_ascii_case("head") {
            RequestMethod::Head
        } else if raw_method.eq_ignore_ascii_case("options") {
            RequestMethod::Options
        } else if raw_method.eq_ignore_ascii_case("patch") {
            RequestMethod::Patch
        } else if raw_method.eq_ignore_ascii_case("post") {
            RequestMethod::Post
        } else if raw_method.eq_ignore_ascii_case("put") {
            RequestMethod::Put
        } else {
            RequestMethod::Other
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_detailed_parameters(
        raw_type: &str,
        url: &str,
        schema: &str,
        hostname: &str,
        domain: &str,
        source_hostname: &str,
        third_party: bool,
        original_url: String,
        method: Option<RequestMethod>,
    ) -> Request {
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

        let source_hostname_hashes = Self::suffix_hostname_hashes(source_hostname);

        let url_lower_cased = url.to_ascii_lowercase();

        Request {
            request_type,
            method,
            url: url.to_owned(),
            url_lower_cased: url_lower_cased.to_owned(),
            hostname: hostname.to_owned(),
            domain: domain.to_owned(),
            source_hostname: source_hostname.to_owned(),
            request_tokens: calculate_tokens(&url_lower_cased),
            source_hostname_hashes,
            destination_suffix_hashes: OnceLock::new(),
            destination_entity_hashes: OnceLock::new(),
            is_third_party: third_party,
            is_http,
            is_https,
            is_supported,
            original_url,
        }
    }

    /// Construct a new [`Request`].
    pub fn new(
        url: &str,
        source_url: &str,
        request_type: &str,
        method: &str,
    ) -> Result<Request, RequestError> {
        if let Some(parsed_url) = url_parser::parse_url(url) {
            let parsed_method = Self::parse_method(method);
            if let Some(parsed_source) = url_parser::parse_url(source_url) {
                let source_domain = parsed_source.domain();

                let third_party = source_domain != parsed_url.domain();

                Ok(Request::from_detailed_parameters(
                    request_type,
                    &parsed_url.url,
                    parsed_url.schema(),
                    parsed_url.hostname(),
                    parsed_url.domain(),
                    parsed_source.hostname(),
                    third_party,
                    url.to_string(),
                    parsed_method,
                ))
            } else {
                Ok(Request::from_detailed_parameters(
                    request_type,
                    &parsed_url.url,
                    parsed_url.schema(),
                    parsed_url.hostname(),
                    parsed_url.domain(),
                    "",
                    true,
                    url.to_string(),
                    parsed_method,
                ))
            }
        } else {
            Err(RequestError::HostnameParseError)
        }
    }

    /// If you're building a [`Request`] in a context that already has access to parsed
    /// representations of the input URLs, you can use this constructor to avoid extra lookups from
    /// the public suffix list. Take care to pass data correctly.
    pub fn preparsed(
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party: bool,
        method: &str,
    ) -> Request {
        let splitter = memchr::memchr(b':', url.as_bytes()).unwrap_or(0);
        let schema: &str = &url[..splitter];
        let (domain_start, domain_end) = url_parser::get_host_domain(hostname);
        let domain = &hostname[domain_start..domain_end];

        Request::from_detailed_parameters(
            request_type,
            url,
            schema,
            hostname,
            domain,
            source_hostname,
            third_party,
            url.to_string(),
            Self::parse_method(method),
        )
    }

    #[cfg(test)]
    pub(crate) fn destination_hashes_initialized(&self) -> bool {
        self.destination_suffix_hashes.get().is_some()
            || self.destination_entity_hashes.get().is_some()
    }
}

fn calculate_tokens(url_lower_cased: &str) -> Vec<utils::Hash> {
    let mut tokens = utils::TokensBuffer::default();
    utils::tokenize_pooled(url_lower_cased, &mut tokens);
    // Add zero token as a fallback to wildcard rule bucket
    tokens.push(0);
    tokens.into_iter().collect()
}

#[cfg(test)]
#[path = "../tests/unit/request.rs"]
mod unit_tests;
