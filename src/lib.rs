
use addr::{DomainName};
// use url::{Url};
#[macro_use] extern crate matches;

mod parser;

pub fn get_host_domain(host: &str) -> String {
    match host.parse::<DomainName>() {
        Err(_e) => String::from(host),
        Ok(domain) => String::from(domain.root().to_str())
    }
}

pub fn get_url_host(url: &str) -> Option<String> {
  parser::Hostname::parse(&url)
    .ok() // convert to Option
    .and_then(|p| p.host_str().map(String::from))
    
}

pub fn get_url_domain(url: &str) -> Option<String> {
    get_url_host(&url).map(|host| get_host_domain(&host))
}
