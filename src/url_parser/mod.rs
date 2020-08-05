mod parser;
use crate::request::Request;
use addr::DomainName;
// mod parser_regex;

pub struct RequestUrl {
    pub url: String,
    schema_end: usize,
    pub hostname_pos: (usize, usize),
    domain: (usize, usize),
}

impl RequestUrl {
    pub fn schema(&self) -> &str {
        &self.url[..self.schema_end]
    }
    pub fn hostname(&self) -> &str {
        &self.url[self.hostname_pos.0..self.hostname_pos.1]
    }
    pub fn domain(&self) -> &str {
        &self.url[self.hostname_pos.0 + self.domain.0 .. self.hostname_pos.0 + self.domain.1]
    }
}

pub trait UrlParser {
    /// Return the string representation of the host (domain or IP address) for this URL, if any together with the URL.
    ///
    /// As part of hostname parsing, punycode decoding is used to convert URLs with UTF characters to plain ASCII ones.
    /// Serialisation then contains this decoded URL that is used for further matching.
    fn parse_url(url: &str) -> Option<RequestUrl>;
}

impl UrlParser for Request {
    fn parse_url(url: &str) -> Option<RequestUrl> {
        let parsed = parser::Hostname::parse(&url).ok();
        parsed.and_then(|h| {
            match h.host_str() {
                Some(_host) => Some(RequestUrl {
                    url: h.url_str().to_owned(),
                    schema_end: h.scheme_end,
                    hostname_pos: (h.host_start, h.host_end),
                    domain: get_host_domain(&h.url_str()[h.host_start..h.host_end])
                }),
                _ => None
            }
        })

    }
}

pub fn get_host_domain(host: &str) -> (usize, usize) {
    if host.is_empty() {
        (0, 0)
    } else {
        match host.parse::<DomainName>() {
            Err(_e) => (0, host.len()),
            Ok(domain) => {
                let root = domain.root();
                let domain_str = root.to_str();
                let domain_len = domain_str.len();
                let host_len = host.len();
                (host_len - domain_len, host_len)
            }
        }
    }
}
