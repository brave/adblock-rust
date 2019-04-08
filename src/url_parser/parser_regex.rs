use regex::Regex;

#[inline]
pub fn get_hostname_regex(url: &str) -> Option<&str> {
    lazy_static! {
        static ref HOSTNAME_REGEX_STR: &'static str = concat!(
            r"[a-z][a-z0-9+\-.]*://",                       // Scheme
            r"(?:[a-z0-9\-._~%!$&'()*+,;=]+@)?",              // User
            r"(?P<host>[a-z0-9\-._~%]+",                            // Named host
            r"|\[[a-f0-9:.]+\]",                            // IPv6 host
            r"|\[v[a-f0-9][a-z0-9\-._~%!$&'()*+,;=:]+\])",  // IPvFuture host
            // r"(?::[0-9]+)?",                                  // Port
            // r"(?:/[a-z0-9\-._~%!$&'()*+,;=:@]+)*/?",          // Path
            // r"(?:\?[a-z0-9\-._~%!$&'()*+,;=:@/?]*)?",         // Query
            // r"(?:\#[a-z0-9\-._~%!$&'()*+,;=:@/?]*)?",         // Fragment
        );
        static ref HOST_REGEX: Regex = Regex::new(&HOSTNAME_REGEX_STR).unwrap();
    }

    HOST_REGEX.captures(url).and_then(|c| c.name("host")).map(|m| m.as_str())
}


#[inline]
pub fn get_url_host(url: &str) -> Option<String> {
    let decode_flags = idna::uts46::Flags {
        use_std3_ascii_rules: true,
        transitional_processing: true,
        verify_dns_length: true,
    };
    parser::get_hostname_regex(&url)
        .and_then(|h| {
            if h.is_ascii() {
                Some(String::from(h))
            } else {
                idna::uts46::to_ascii(&h, decode_flags).ok()
            }
        })
}


#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    // pattern
    fn parses_hostname() {
        assert_eq!(get_hostname_regex("http://example.foo.edu.au"), Some("example.foo.edu.au"));
        assert_eq!(get_hostname_regex("http://example.foo.edu.sh"), Some("example.foo.edu.sh"));
        assert_eq!(get_hostname_regex("http://example.foo.nom.br"), Some("example.foo.nom.br"));
        assert_eq!(get_hostname_regex("http://example.foo.nom.br:80/"), Some("example.foo.nom.br"));
        assert_eq!(get_hostname_regex("http://example.foo.nom.br:8080/hello?world=true"), Some("example.foo.nom.br"));
        assert_eq!(get_hostname_regex("http://example.foo.nom.br/hello#world"), Some("example.foo.nom.br"));
        assert_eq!(get_hostname_regex("http://127.0.0.1:80"), Some("127.0.0.1"));
        assert_eq!(get_hostname_regex("http://[2001:470:20::2]"), Some("[2001:470:20::2]"));
        assert_eq!(get_hostname_regex("http://[2001:4860:4860::1:8888]"), Some("[2001:4860:4860::1:8888]"));
    }
}
