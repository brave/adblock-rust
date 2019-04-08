mod parser;
use crate::request::Request;
use addr::DomainName;

pub struct RequestUrl {
    pub url: String,
    schema_end: usize,
    hostname_pos: (usize, usize),
    pub domain: String,
}

impl RequestUrl {
    pub fn schema(&self) -> &str {
        &self.url[..self.schema_end]
    }
    pub fn hostname(&self) -> &str {
        &self.url[self.hostname_pos.0..self.hostname_pos.1]
    }
}

pub trait UrlParser {
    /// Return the string representation of the host (domain or IP address) for this URL, if any together with the URL.
    /// 
    /// As part of hostname parsing, punycode decoding is used to convert URLs with UTF characters to plain ASCII ones.
    /// Serialisation then contains this decoded URL that is used for further matching.
    /// 
    fn get_url_host(url: &str) -> Option<RequestUrl>;
}

impl UrlParser for Request {
    #[inline]
    fn get_url_host(url: &str) -> Option<RequestUrl> {
        let parsed = parser::Hostname::parse(&url).ok();
        parsed.and_then(|h| {
            match h.host_str() {
                Some(_host) => Some(RequestUrl {
                    url: h.url_str(),
                    schema_end: h.scheme_end,
                    hostname_pos: (h.host_start, h.host_end),
                    domain: get_host_domain(&url[h.host_start..h.host_end])
                }),
                _ => None
            }
        })

    }
}

pub fn get_host_domain(host: &str) -> String {
    match host.parse::<DomainName>() {
        Err(_e) => String::from(host),
        Ok(domain) => String::from(domain.root().to_str()),
    }
}

pub fn get_url_domain(url: &str) -> Option<String> {
    Request::get_url_host(&url).map(|parsed_url| parsed_url.domain)
}
