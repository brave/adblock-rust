use regex::Regex;

use crate::utils;
use crate::utils::{Hash};
use crate::request;

pub const TOKENS_BUFFER_SIZE: usize = 200;

#[derive(Debug, PartialEq)]
pub enum FilterError {
    FilterParseError
}

bitflags!{
    struct NETWORK_FILTER_MASK: u32 {
        const fromImage = 1 << 0;
        const fromMedia = 1 << 1;
        const fromObject = 1 << 2;
        const fromOther = 1 << 3;
        const fromPing = 1 << 4;
        const fromScript = 1 << 5;
        const fromStylesheet = 1 << 6;
        const fromSubdocument = 1 << 7;
        const fromWebsocket = 1 << 8; // e.g.: ws, ws
        const fromXmlHttpRequest = 1 << 9;
        const fromFont = 1 << 10;
        const fromHttp = 1 << 11;
        const fromHttps = 1 << 12;
        const isImportant = 1 << 13;
        const matchCase = 1 << 14;
        const fuzzyMatch = 1 << 15;
        
        // Kind of pattern
        const thirdParty = 1 << 16;
        const firstParty = 1 << 17;
        const isRegex = 1 << 18;
        const isLeftAnchor = 1 << 19;
        const isRightAnchor = 1 << 20;
        const isHostnameAnchor = 1 << 21;
        const isException = 1 << 22;
        const isCSP = 1 << 23;

        // "Other" network request types
        const unmatched = 1 << 24;

        const FROM_ANY = Self::fromFont.bits |
            Self::fromImage.bits |
            Self::fromMedia.bits |
            Self::fromObject.bits |
            Self::fromOther.bits |
            Self::fromPing.bits |
            Self::fromScript.bits |
            Self::fromStylesheet.bits |
            Self::fromSubdocument.bits |
            Self::fromWebsocket.bits |
            Self::fromXmlHttpRequest.bits;
    }
}

impl From<request::RequestType> for NETWORK_FILTER_MASK {
    fn from(request_type: request::RequestType) -> NETWORK_FILTER_MASK {
        match request_type {
            request::RequestType::Other         => NETWORK_FILTER_MASK::fromOther,
            request::RequestType::Script        => NETWORK_FILTER_MASK::fromScript,
            request::RequestType::Image         => NETWORK_FILTER_MASK::fromImage,
            request::RequestType::Stylesheet    => NETWORK_FILTER_MASK::fromStylesheet,
            request::RequestType::Object        => NETWORK_FILTER_MASK::fromObject,
            request::RequestType::Subdocument   => NETWORK_FILTER_MASK::fromSubdocument,
            request::RequestType::Ping          => NETWORK_FILTER_MASK::fromPing,
            request::RequestType::Beacon        => NETWORK_FILTER_MASK::fromPing,
            request::RequestType::Xmlhttprequest => NETWORK_FILTER_MASK::fromXmlHttpRequest,
            request::RequestType::Font          => NETWORK_FILTER_MASK::fromFont,
            request::RequestType::Media         => NETWORK_FILTER_MASK::fromMedia,
            request::RequestType::Websocket     => NETWORK_FILTER_MASK::fromWebsocket,
            request::RequestType::Dtd           => NETWORK_FILTER_MASK::fromOther,
            request::RequestType::Fetch         => NETWORK_FILTER_MASK::fromOther,
            request::RequestType::Xlst          => NETWORK_FILTER_MASK::fromOther,
            // TODO: check if the logic is actually correct, TS implementation mapping was non-exhaustive, following options added:
            request::RequestType::Document      => NETWORK_FILTER_MASK::unmatched,
            request::RequestType::Csp           => NETWORK_FILTER_MASK::unmatched
        }
    }
}

pub struct NetworkFilter {
    mask: NETWORK_FILTER_MASK,
    pub filter: Option<String>,
    pub opt_domains: Option<Vec<Hash>>,
    pub opt_not_domains: Option<Vec<Hash>>,
    pub redirect: Option<String>,
    pub hostname: Option<String>,
    pub csp: Option<String>,
    pub bug: Option<u32>,

    // Set only in debug mode
    pub debug: bool,
    pub raw_line: Option<String>,

    // Lazy attributes
    pub id: Option<Hash>,
    fuzzy_signature: Option<Vec<Hash>>,
    regex: Option<Regex>
}

impl NetworkFilter {
    pub fn parse(line: &str, debug: bool) -> Result<NetworkFilter, FilterError> {
         unimplemented!();
    }

    pub fn matches(&self, request: &request::Request) -> bool {
        check_options(&self, request) && check_pattern(&self, request)
    }

    pub fn to_string() -> String {
        unimplemented!();
    }

    pub fn get_id(&self) -> Hash {
        compute_filter_id(
            self.csp.as_ref().map(String::as_str),
            self.mask,
            self.filter.as_ref().map(String::as_str),
            self.hostname.as_ref().map(String::as_str),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref())
    }

    pub fn get_regex(&mut self) -> &Regex {
        if self.regex.is_none() {
            self.regex = match self.filter {
                Some(ref filter) if self.is_regex() => Some(compile_regex(filter, self.is_right_anchor(), self.is_left_anchor())), // compile regex
                _ => Some(Regex::new("").unwrap())
            }
        }
        &self.regex.as_ref().unwrap()
    }

    pub fn get_fuzzy_signature(&mut self) -> &Vec<Hash> {
        if self.fuzzy_signature.is_none() {
            self.fuzzy_signature = match self.filter {
                Some(ref filter) if self.is_fuzzy() => Some(utils::create_fuzzy_signature(filter)),
                _ => Some(vec![])
            }
        }
        self.fuzzy_signature.as_ref().unwrap()
    }

    pub fn get_tokens(&self) -> Vec<Vec<Hash>> {
        let mut tokens: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);
        
        // If there is only one domain and no domain negation, we also use this
        // domain as a token.
        if self.opt_domains.is_some() &&
            self.opt_not_domains.is_none() &&
            self.opt_domains.as_ref().map(|d| d.len()) == Some(1) {
                let domains = self.opt_domains.as_ref().unwrap();
                let domain = domains.get(0).unwrap();
                tokens.push(*domain)
            }

        // Get tokens from filter
        self.filter.as_ref().map(|filter| {
            let skip_last_token = self.is_plain() && !self.is_right_anchor() && !self.is_fuzzy();
            let skip_first_token = self.is_right_anchor();
            let mut filter_tokens = utils::tokenize_filter(filter, skip_first_token, skip_last_token);
            
            tokens.append(&mut filter_tokens);
        });

        // Append tokens from hostname, if any
        self.hostname.as_ref().map(|hostname| {
            let mut hostname_tokens = utils::tokenize(&hostname);
            tokens.append(&mut hostname_tokens);
        });

        // If we got no tokens for the filter/hostname part, then we will dispatch
        // this filter in multiple buckets based on the domains option.
        if tokens.len() == 0 &&
            self.opt_domains.is_some() &&
            self.opt_not_domains.is_none() {
                self.opt_domains.as_ref().unwrap_or(&vec![])
                    .iter()
                    .map(|&d| vec![d])
                    .collect()
        } else {
            // Add optional token for protocol
            if self.from_http() && !self.from_https() {
                tokens.push(utils::fast_hash("http"));
            } else if self.from_https() && !self.from_http() {
                tokens.push(utils::fast_hash("https"));
            }
            tokens.shrink_to_fit();
            vec![tokens]
        }
    }

    pub fn is_cpt_allowed(&self, cpt: request::RequestType) -> bool {
        match NETWORK_FILTER_MASK::from(cpt) {
            NETWORK_FILTER_MASK::unmatched => self.from_any(),
            mask => self.mask.contains(mask)
        }
    }


    #[inline] fn get_cpt_mask(&self) -> NETWORK_FILTER_MASK { self.mask & NETWORK_FILTER_MASK::FROM_ANY }
    #[inline] fn is_fuzzy(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::fuzzyMatch) }
    #[inline] fn is_exception(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isException) }
    #[inline] fn is_hostname_anchor(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isHostnameAnchor) }
    #[inline] fn is_right_anchor(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isRightAnchor) }
    #[inline] fn is_left_anchor(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isLeftAnchor) }
    #[inline] fn match_case(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::matchCase) }
    #[inline] fn is_important(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isImportant) }
    #[inline] fn is_regex(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isRegex) }
    #[inline] fn is_plain(&self) -> bool { !self.is_regex() }
    #[inline] fn is_csp(&self) -> bool { self.mask.contains(NETWORK_FILTER_MASK::isCSP) }
    #[inline] fn has_bug(&self) -> bool { self.bug.is_some() }
    #[inline] fn from_any(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::FROM_ANY) }
    #[inline] fn third_party(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::thirdParty) }
    #[inline] fn first_party(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::firstParty) }
    #[inline] fn from_image(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromImage) }
    #[inline] fn from_media(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromMedia) }
    #[inline] fn from_object(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromObject) }
    #[inline] fn from_other(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromOther) }
    #[inline] fn from_ping(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromPing) }
    #[inline] fn from_script(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromScript) }
    #[inline] fn from_stylesheet(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromStylesheet) }
    #[inline] fn from_subdocument(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromSubdocument) }
    #[inline] fn from_websocket(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromWebsocket) }
    #[inline] fn from_http(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromHttp) }
    #[inline] fn from_https(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromHttps) }
    #[inline] fn from_xmlHttpRequest(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromXmlHttpRequest) }
    #[inline] fn from_font(&self) -> bool { self.get_cpt_mask().contains(NETWORK_FILTER_MASK::fromFont) }


}

// ---------------------------------------------------------------------------
// Filter parsing
// ---------------------------------------------------------------------------

fn compute_filter_id(
  csp: Option<&str>,
  mask: NETWORK_FILTER_MASK,
  filter: Option<&str>,
  hostname: Option<&str>,
  opt_domains: Option<&Vec<Hash>>,
  opt_not_domains: Option<&Vec<Hash>>
) -> Hash {
    let mut hash: Hash = (5408 * 33) ^ (mask.bits as Hash);

    csp.map(|s| {
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    });

    opt_domains.map(|domains| {
        for d in domains {
            hash = hash.wrapping_mul(33) ^ d;
        }
    });

    opt_not_domains.map(|domains| {
        for d in domains {
            hash = hash.wrapping_mul(33) ^ d;
        }
    });

    filter.map(|s| {
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    });

    hostname.map(|s| {
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    });

    hash
}


/**
 * Compiles a filter pattern to a regex. This is only performed *lazily* for
 * filters containing at least a * or ^ symbol. Because Regexes are expansive,
 * we try to convert some patterns to plain filters.
 */
fn compile_regex(filter_str: &str, is_right_anchor: bool, is_left_anchor: bool) -> Regex {
  lazy_static! {
    // Escape special regex characters: |.$+?{}()[]\
    static ref SPECIAL_RE: Regex = Regex::new(r"([|.$+?{}()[\]\\])").unwrap();
    // * can match anything
    static ref WILDCARD_RE: Regex = Regex::new(r"\*").unwrap();
    // ^ can match any separator or the end of the pattern
    static ref ANCHOR_RE: Regex = Regex::new(r"\^").unwrap();
  }
  
  let repl_special = SPECIAL_RE.replace_all(&filter_str, "\\$1");
  let repl_wildcard = WILDCARD_RE.replace_all(&repl_special, ".*");
  let repl_anchor = ANCHOR_RE.replace_all(&repl_wildcard, "(?:[^\\w\\d_.%-]|$)");

  // Should match start or end of url
  let left_anchor = if is_left_anchor { "^" } else { "" };
  let right_anchor = if is_right_anchor { "$" } else { "" };
  let filter = format!("{}{}{}", left_anchor, repl_anchor, right_anchor);

  Regex::new(&filter).unwrap()
}

/**
 * Check if the sub-string contained between the indices start and end is a
 * regex filter (it contains a '*' or '^' char). Here we are limited by the
 * capability of javascript to check the presence of a pattern between two
 * indices (same for Regex...).
 * // TODO - we could use sticky regex here
 */
fn check_is_regex(filter: &str) -> bool {
  let start_index = filter.find("*");
  let separator_index = filter.find("^");
  start_index.is_some() || separator_index.is_some()
}

/**
 * Handle hostname anchored filters, given 'hostname' from ||hostname and
 * request's hostname, check if there is a match. This is tricky because filters
 * authors rely and different assumption. We can have prefix of suffix matches
 * of anchor.
 */
fn is_anchored_by_hostname(filter_hostname: &str, hostname: &str) -> bool {
    unimplemented!();
}

fn get_url_after_hostname<'a>(url: &'a str, hostname: &str) -> &'a str {
    let start = url.find(&hostname).unwrap_or(url.len());
    &url[start..url.len()]
}


// ---------------------------------------------------------------------------
// Filter matching
// ---------------------------------------------------------------------------

// pattern$fuzzy
fn check_pattern_fuzzy_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// pattern
fn check_pattern_plain_filter_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// pattern|
fn check_pattern_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// |pattern
fn check_pattern_left_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// |pattern|
fn check_pattern_left_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// pattern*^
fn check_pattern_regex_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// ||pattern*^
fn check_pattern_hostname_anchor_regex_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// ||pattern|
fn check_pattern_hostname_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// |||pattern|
fn check_pattern_hostname_left_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// ||pattern + left-anchor => This means that a plain pattern needs to appear
// exactly after the hostname, with nothing in between.
fn check_pattern_hostname_left_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// ||pattern
fn check_pattern_hostname_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

// ||pattern$fuzzy
fn check_pattern_hostname_anchor_fuzzy_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

/**
 * Specialize a network filter depending on its type. It allows for more
 * efficient matching function.
 */
fn check_pattern(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}

fn check_options(filter: &NetworkFilter, request: &request::Request) -> bool {
    unimplemented!();
}