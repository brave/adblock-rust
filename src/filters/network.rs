use idna;
use regex::Regex;
use regex::RegexSet;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::request;
use crate::utils;
use crate::utils::Hash;
use std::cell::RefCell;
use std::rc::Rc;
use twoway;

pub const TOKENS_BUFFER_SIZE: usize = 200;

#[derive(Debug, PartialEq)]
pub enum FilterError {
    FilterParseError,
    NegatedBadFilter,
    NegatedImportant,
    NegatedOptionMatchCase,
    NegatedExplicitCancel,
    NegatedRedirection,
    NegatedTag,
    EmptyRedirection,
    UnrecognisedOption,
    NoRegex,
    FullRegexUnsupported,
    RegexParsingError(regex::Error),
    PunycodeError,
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct NetworkFilterMask: u32 {
        const FROM_IMAGE = 1; // 1 << 0;
        const FROM_MEDIA = 1 << 1;
        const FROM_OBJECT = 1 << 2;
        const FROM_OTHER = 1 << 3;
        const FROM_PING = 1 << 4;
        const FROM_SCRIPT = 1 << 5;
        const FROM_STYLESHEET = 1 << 6;
        const FROM_SUBDOCUMENT = 1 << 7;
        const FROM_WEBSOCKET = 1 << 8; // e.g.: ws, ws
        const FROM_XMLHTTPREQUEST = 1 << 9;
        const FROM_FONT = 1 << 10;
        const FROM_HTTP = 1 << 11;
        const FROM_HTTPS = 1 << 12;
        const IS_IMPORTANT = 1 << 13;
        const MATCH_CASE = 1 << 14;
        const FUZZY_MATCH = 1 << 15;
        const THIRD_PARTY = 1 << 16;
        const FIRST_PARTY = 1 << 17;
        const EXPLICIT_CANCEL = 1 << 26;
        const BAD_FILTER = 1 << 27;

        // Kind of pattern
        const IS_REGEX = 1 << 18;
        const IS_LEFT_ANCHOR = 1 << 19;
        const IS_RIGHT_ANCHOR = 1 << 20;
        const IS_HOSTNAME_ANCHOR = 1 << 21;
        const IS_EXCEPTION = 1 << 22;
        const IS_CSP = 1 << 23;
        const IS_COMPLETE_REGEX = 1 << 24;
        const IS_HOSTNAME_REGEX = 1 << 28;

        // "Other" network request types
        const UNMATCHED = 1 << 25;

        const FROM_ANY = Self::FROM_FONT.bits |
            Self::FROM_IMAGE.bits |
            Self::FROM_MEDIA.bits |
            Self::FROM_OBJECT.bits |
            Self::FROM_OTHER.bits |
            Self::FROM_PING.bits |
            Self::FROM_SCRIPT.bits |
            Self::FROM_STYLESHEET.bits |
            Self::FROM_SUBDOCUMENT.bits |
            Self::FROM_WEBSOCKET.bits |
            Self::FROM_XMLHTTPREQUEST.bits;

        // Unless filter specifies otherwise, all these options are set by default
        const DEFAULT_OPTIONS = Self::FROM_ANY.bits |
            Self::FROM_HTTP.bits |
            Self::FROM_HTTPS.bits |
            Self::THIRD_PARTY.bits |
            Self::FIRST_PARTY.bits;

        // Careful with checking for NONE - will always match
        const NONE = 0;
    }
}

#[derive(Debug, Clone)]
pub enum CompiledRegex {
    Compiled(Regex),
    CompiledSet(RegexSet),
    MatchAll,
    RegexParsingError(regex::Error),
}

impl CompiledRegex {
    pub fn is_match(&self, pattern: &str) -> bool {
        match &self {
            CompiledRegex::MatchAll => true, // simple case for matching everything, e.g. for empty filter
            CompiledRegex::RegexParsingError(_e) => false, // no match if regex didn't even compile
            CompiledRegex::Compiled(r) => r.is_match(pattern),
            CompiledRegex::CompiledSet(r) => {
                // let matches: Vec<_> = r.matches(pattern).into_iter().collect();
                // println!("Matching {} against RegexSet: {:?}", pattern, matches);
                r.is_match(pattern)
            }
        }
    }

    pub fn to_string(&self) -> String {
        match &self {
            CompiledRegex::MatchAll => String::from(".*"), // simple case for matching everything, e.g. for empty filter
            CompiledRegex::RegexParsingError(_e) => String::from("ERROR"), // no match if regex didn't even compile
            CompiledRegex::Compiled(r) => String::from(r.as_str()),
            CompiledRegex::CompiledSet(r) => r.patterns().join(" | "),
        }
    }
}

impl fmt::Display for NetworkFilterMask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:b}", &self)
    }
}

impl From<&request::RequestType> for NetworkFilterMask {
    fn from(request_type: &request::RequestType) -> NetworkFilterMask {
        match request_type {
            request::RequestType::Beacon => NetworkFilterMask::FROM_PING,
            request::RequestType::Csp => NetworkFilterMask::UNMATCHED,
            request::RequestType::Document => NetworkFilterMask::UNMATCHED,
            request::RequestType::Dtd => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Fetch => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Font => NetworkFilterMask::FROM_FONT,
            request::RequestType::Image => NetworkFilterMask::FROM_IMAGE,
            request::RequestType::Media => NetworkFilterMask::FROM_MEDIA,
            request::RequestType::Object => NetworkFilterMask::FROM_OBJECT,
            request::RequestType::Other => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Ping => NetworkFilterMask::FROM_PING,
            request::RequestType::Script => NetworkFilterMask::FROM_SCRIPT,
            request::RequestType::Stylesheet => NetworkFilterMask::FROM_STYLESHEET,
            request::RequestType::Subdocument => NetworkFilterMask::FROM_SUBDOCUMENT,
            request::RequestType::Websocket => NetworkFilterMask::FROM_WEBSOCKET,
            request::RequestType::Xlst => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Xmlhttprequest => NetworkFilterMask::FROM_XMLHTTPREQUEST,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterPart {
    Empty,
    Simple(String),
    AnyOf(Vec<String>),
}

impl FilterPart {
    pub fn string_view(&self) -> Option<String> {
        match &self {
            FilterPart::Empty => None,
            FilterPart::Simple(s) => Some(s.clone()),
            FilterPart::AnyOf(s) => Some(s.join("|")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFilter {
    pub mask: NetworkFilterMask,
    pub filter: FilterPart,
    pub opt_domains: Option<Vec<Hash>>,
    pub opt_not_domains: Option<Vec<Hash>>,
    pub redirect: Option<String>,
    pub hostname: Option<String>,
    pub csp: Option<String>,
    pub bug: Option<u32>,
    pub tag: Option<String>,

    pub raw_line: Option<String>,

    pub id: Hash,
    pub fuzzy_signature: Option<Vec<Hash>>,

    // All domain option values (their hashes) OR'ed together to quickly dismiss mis-matches
    pub opt_domains_union: Option<Hash>,
    pub opt_not_domains_union: Option<Hash>,

    // Regex compild lazily, using "Interior Mutability"
    // Arc (Atomic Reference Counter) allows for cloned NetworkFilters
    // to point to the same RwLock and what is inside.
    // RwLock allows for concurrent access when reading as well as writing
    // from the inside.
    // When the Regex hasn't been compiled, <None> is stored, afterwards Arc to Some<CompiledRegex>
    // to avoid expensive cloning of the Regex itself.
    #[serde(skip_serializing, skip_deserializing)]
    regex: Rc<RefCell<Option<Rc<CompiledRegex>>>>
}

impl NetworkFilter {
    #[allow(clippy::cognitive_complexity)]
    pub fn parse(line: &str, debug: bool) -> Result<NetworkFilter, FilterError> {
        // Represent options as a bitmask
        let mut mask: NetworkFilterMask = NetworkFilterMask::THIRD_PARTY
            | NetworkFilterMask::FIRST_PARTY
            | NetworkFilterMask::FROM_HTTPS
            | NetworkFilterMask::FROM_HTTP;

        // Temporary masks for positive (e.g.: $script) and negative (e.g.: $~script)
        // content type options.
        let mut cpt_mask_positive: NetworkFilterMask = NetworkFilterMask::NONE;
        let mut cpt_mask_negative: NetworkFilterMask = NetworkFilterMask::FROM_ANY;

        let mut hostname: Option<String> = None;

        let mut opt_domains: Option<Vec<Hash>> = None;
        let mut opt_not_domains: Option<Vec<Hash>> = None;
        let mut opt_domains_union: Option<Hash> = None;
        let mut opt_not_domains_union: Option<Hash> = None;

        let mut redirect: Option<String> = None;
        let mut csp: Option<String> = None;
        let mut bug: Option<u32> = None;
        let mut tag: Option<String> = None;

        // Start parsing
        let mut filter_index_start: usize = 0;
        let mut filter_index_end: usize = line.len();

        // @@filter == Exception
        if line.starts_with("@@") {
            filter_index_start += 2;
            mask.set(NetworkFilterMask::IS_EXCEPTION, true);
        }

        // filter$options == Options
        // ^     ^
        // |     |
        // |     optionsIndex
        // filterIndexStart
        let maybe_options_index: Option<usize> = twoway::rfind_str(&line, "$");

        if let Some(options_index) = maybe_options_index {
            // Parse options and set flags
            filter_index_end = options_index;

            // Parse Options
            let raw_options = &line[filter_index_end + 1..];
            let options = raw_options.split(',');
            for raw_option in options {
                // Check for negation: ~option
                let negation = raw_option.starts_with('~');
                let maybe_negated_option = raw_option.trim_start_matches('~');

                // Check for options: option=value1|value2
                let mut option_and_values = maybe_negated_option.splitn(2, '=');
                let (option, value) = (
                    option_and_values.next().unwrap(),
                    option_and_values.next().unwrap_or_default(),
                );

                match (option, negation) {
                    ("domain", _) => {
                        let mut option_values: Vec<&str> = value.split('|').collect();
                        // Some rules have duplicate domain options - avoid including duplicates
                        // Benchmarking doesn't indicate signficant performance degradation across the entire easylist
                        option_values.sort();
                        option_values.dedup();
                        let mut opt_domains_array: Vec<Hash> = vec![];
                        let mut opt_not_domains_array: Vec<Hash> = vec![];

                        for option_value in option_values {
                            if option_value.starts_with('~') {
                                let domain = &option_value[1..];
                                let domain_hash = utils::fast_hash(&domain);
                                opt_not_domains_array.push(domain_hash);
                            } else {
                                let domain_hash = utils::fast_hash(&option_value);
                                opt_domains_array.push(domain_hash);
                            }
                        }

                        if !opt_domains_array.is_empty() {
                            opt_domains_array.sort();
                            opt_domains_union = Some(opt_domains_array.iter().fold(0, |acc, x| acc | x));
                            opt_domains = Some(opt_domains_array);
                        }
                        if !opt_not_domains_array.is_empty() {
                            opt_not_domains_array.sort();
                            opt_not_domains_union = Some(opt_not_domains_array.iter().fold(0, |acc, x| acc | x));
                            opt_not_domains = Some(opt_not_domains_array);
                        }
                    }
                    ("badfilter", false) => mask.set(NetworkFilterMask::BAD_FILTER, true),
                    ("badfilter", true) => return Err(FilterError::NegatedBadFilter),
                    // Note: `negation` should always be `false` here.
                    ("important", true) => return Err(FilterError::NegatedImportant),
                    ("important", false) => mask.set(NetworkFilterMask::IS_IMPORTANT, true),
                    // Note: `negation` should always be `false` here.
                    ("match-case", true) => return Err(FilterError::NegatedOptionMatchCase),
                    ("match-case", false) => mask.set(NetworkFilterMask::MATCH_CASE, true),
                    // ~third-party means we should clear the flag
                    ("third-party", true) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("third-party", false) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    ("3p", true) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("3p", false) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    // ~first-party means we should clear the flag
                    ("first-party", true) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    // first-party means ~third-party
                    ("first-party", false) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("1p", true) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    ("1p", false) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("fuzzy", _) => mask.set(NetworkFilterMask::FUZZY_MATCH, true),
                    ("collapse", _) => {}
                    ("bug", _) => bug = value.parse::<u32>().ok(),
                    ("tag", false) => tag = Some(String::from(value)),
                    ("tag", true) => return Err(FilterError::NegatedTag),
                    // Negation of redirection doesn't make sense
                    ("redirect", true) => return Err(FilterError::NegatedRedirection),
                    ("redirect", false) => {
                        // Ignore this filter if no redirection resource is specified
                        if value.is_empty() {
                            return Err(FilterError::EmptyRedirection);
                        }

                        redirect = Some(String::from(value));
                    }
                    ("explicitcancel", true) => return Err(FilterError::NegatedExplicitCancel),
                    ("explicitcancel", false) => mask.set(NetworkFilterMask::EXPLICIT_CANCEL, true),
                    ("csp", _) => {
                        mask.set(NetworkFilterMask::IS_CSP, true);
                        if !value.is_empty() {
                            csp = Some(String::from(value));
                        }
                    }
                    (_, negation) => {
                        // Handle content type options separatly
                        let mut option_mask = NetworkFilterMask::NONE;
                        match option {
                            "image" => option_mask.set(NetworkFilterMask::FROM_IMAGE, true),
                            "media" => option_mask.set(NetworkFilterMask::FROM_MEDIA, true),
                            "object" | "object-subrequest" => option_mask.set(NetworkFilterMask::FROM_OBJECT, true),
                            "other" => option_mask.set(NetworkFilterMask::FROM_OTHER, true),
                            "ping" | "beacon" => option_mask.set(NetworkFilterMask::FROM_PING, true),
                            "script" => option_mask.set(NetworkFilterMask::FROM_SCRIPT, true),
                            "css" | "stylesheet" => option_mask.set(NetworkFilterMask::FROM_STYLESHEET, true),
                            "frame" | "subdocument" => option_mask.set(NetworkFilterMask::FROM_SUBDOCUMENT, true),
                            "xhr" | "xmlhttprequest" => option_mask.set(NetworkFilterMask::FROM_XMLHTTPREQUEST, true),
                            "websocket" => option_mask.set(NetworkFilterMask::FROM_WEBSOCKET, true),
                            "font" => option_mask.set(NetworkFilterMask::FROM_FONT, true),
                            _ => return Err(FilterError::UnrecognisedOption),
                        }

                        // We got a valid cpt option, update mask
                        if negation {
                            cpt_mask_negative.set(option_mask, false);
                        } else {
                            cpt_mask_positive.set(option_mask, true);
                        }
                    }
                }
            }

            // End of option parsing
            // --------------------------------------------------------------------- //
        }

        if cpt_mask_positive.is_empty() {
            mask |= cpt_mask_negative;
        } else if cpt_mask_negative.contains(NetworkFilterMask::FROM_ANY) {
            mask |= cpt_mask_positive;
        } else {
            mask |= cpt_mask_positive & cpt_mask_negative;
        }

        // Identify kind of pattern

        if line[filter_index_start..].starts_with("||") {
            mask.set(NetworkFilterMask::IS_HOSTNAME_ANCHOR, true);
            filter_index_start += 2;
        } else if line[filter_index_start..].starts_with('|') {
            mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
            filter_index_start += 1;
        }

        // Deal with hostname pattern
        if filter_index_end > 0 && filter_index_end > filter_index_start && line[filter_index_end - 1..].starts_with('|') {
            mask.set(NetworkFilterMask::IS_RIGHT_ANCHOR, true);
            filter_index_end -= 1;
        }

        let is_regex = check_is_regex(&line[filter_index_start..filter_index_end]);
        mask.set(NetworkFilterMask::IS_REGEX, is_regex);

        if line[filter_index_start..filter_index_end].starts_with('/')
            && line[filter_index_start..filter_index_end].ends_with('/')
        {
            #[cfg(feature = "full-regex-handling")]
            {
                mask.set(NetworkFilterMask::IS_COMPLETE_REGEX, true);
            }

            #[cfg(not(feature = "full-regex-handling"))]
            {
                return Err(FilterError::FullRegexUnsupported);
            }
        }

        if mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR) {
            if is_regex {
                // Split at the first '/', '*' or '^' character to get the hostname
                // and then the pattern.
                // TODO - this could be made more efficient if we could match between two
                // indices. Once again, we have to do more work than is really needed.
                lazy_static! {
                    static ref SEPARATOR: Regex = Regex::new("[/^*]").unwrap();
                }
                if let Some(first_separator) = SEPARATOR.find(line) {
                    let first_separator_start = first_separator.start();
                    // NOTE: `first_separator` shall never be -1 here since `IS_REGEX` is true.
                    // This means there must be at least an occurrence of `*` or `^`
                    // somewhere.

                    // If the first separator is a wildcard, included in in hostname
                    if first_separator_start < line.len() && line[first_separator_start..=first_separator_start].starts_with('*') {
                        mask.set(NetworkFilterMask::IS_HOSTNAME_REGEX, true);
                    }

                    hostname = Some(String::from(&line[filter_index_start..first_separator_start]));
                    filter_index_start = first_separator_start;

                    // If the only symbol remaining for the selector is '^' then ignore it
                    // but set the filter as right anchored since there should not be any
                    // other label on the right
                    if filter_index_end - filter_index_start == 1
                        && line[filter_index_start..].starts_with('^')
                    {
                        mask.set(NetworkFilterMask::IS_REGEX, false);
                        filter_index_start = filter_index_end;
                        mask.set(NetworkFilterMask::IS_RIGHT_ANCHOR, true);
                    } else {
                        mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
                        mask.set(
                            NetworkFilterMask::IS_REGEX,
                            check_is_regex(&line[filter_index_start..filter_index_end]),
                        );
                    }
                }
            } else {
                // Look for next /
                let slash_index = twoway::find_str(&line[filter_index_start..], "/");
                slash_index
                    .map(|i| {
                        hostname = Some(String::from(
                            &line[filter_index_start..filter_index_start + i],
                        ));
                        filter_index_start += i;
                        mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
                    })
                    .or_else(|| {
                        hostname = Some(String::from(&line[filter_index_start..filter_index_end]));
                        filter_index_start = filter_index_end;
                        None
                    });
            }
        }

        // Remove trailing '*'
        if filter_index_end - filter_index_start > 0
            && line[filter_index_end - 1..].starts_with('*')
        {
            filter_index_end -= 1;
        }

        // Remove leading '*' if the filter is not hostname anchored.
        if filter_index_end - filter_index_start > 0 && line[filter_index_start..].starts_with('*')
        {
            mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
            filter_index_start += 1;
        }

        // Transform filters on protocol (http, https, ws)
        if mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR) {
            if filter_index_end - filter_index_start == 5
                && line[filter_index_start..].starts_with("ws://")
            {
                mask.set(NetworkFilterMask::FROM_WEBSOCKET, true);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 7
                && line[filter_index_start..].starts_with("http://")
            {
                mask.set(NetworkFilterMask::FROM_HTTP, true);
                mask.set(NetworkFilterMask::FROM_HTTPS, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 8
                && line[filter_index_start..].starts_with("https://")
            {
                mask.set(NetworkFilterMask::FROM_HTTPS, true);
                mask.set(NetworkFilterMask::FROM_HTTP, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 8
                && line[filter_index_start..].starts_with("http*://")
            {
                mask.set(NetworkFilterMask::FROM_HTTPS, true);
                mask.set(NetworkFilterMask::FROM_HTTP, true);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            }
        }

        let filter: Option<String> = if filter_index_end - filter_index_start > 0 {
            mask.set(
                NetworkFilterMask::IS_REGEX,
                check_is_regex(&line[filter_index_start..filter_index_end]),
            );
            Some(String::from(&line[filter_index_start..filter_index_end]).to_lowercase())
        } else {
            None
        };

        // TODO: ignore hostname anchor is not hostname provided

        let hostname_decoded = hostname.map(|host| {
            let hostname_normalised = if mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR) {
                host.trim_start_matches("www.")
            } else {
                &host
            };

            let lowercase = hostname_normalised.to_lowercase();
            let mut hostname = String::new();
            if lowercase.is_ascii() {
                hostname.push_str(&lowercase);
            } else {
                let decode_flags = idna::uts46::Flags {
                    use_std3_ascii_rules: true,
                    transitional_processing: true,
                    verify_dns_length: true,
                };
                match idna::uts46::to_ascii(&lowercase, decode_flags) {
                    Ok(x) => hostname.push_str(&x),
                    Err(_) => return Err(FilterError::PunycodeError),
                }
            }
            Ok(hostname)
        });

        let maybe_fuzzy_signature = if mask.contains(NetworkFilterMask::FUZZY_MATCH) {
            filter.as_ref().map(|f| utils::create_fuzzy_signature(f))
        } else {
            None
        };

        Ok(NetworkFilter {
            bug,
            csp,
            filter: if let Some(simple_filter) = filter {
                FilterPart::Simple(simple_filter)
            } else {
                FilterPart::Empty
            },
            hostname: hostname_decoded.map_or(Ok(None), |r| r.map(Some))?,
            mask,
            opt_domains,
            opt_not_domains,
            tag,
            raw_line: if debug {
                Some(String::from(line))
            } else {
                None
            },
            redirect,
            id: utils::fast_hash(&line),
            fuzzy_signature: maybe_fuzzy_signature,
            opt_domains_union,
            opt_not_domains_union,
            regex: Rc::new(RefCell::new(None))
        })
    }

    pub fn to_string(&self) -> String {
        match self.raw_line.as_ref() {
            Some(r) => r.clone(),
            None => String::from(""),
        }
    }

    pub fn get_id_without_badfilter(&self) -> Hash {
        let mut mask = self.mask;
        mask.set(NetworkFilterMask::BAD_FILTER, false);
        compute_filter_id(
            self.csp.as_ref().map(String::as_str),
            mask,
            self.filter.string_view().as_ref().map(|s| s.as_str()),
            self.hostname.as_ref().map(String::as_str),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref(),
        )
    }

    pub fn get_id(&self) -> Hash {
        compute_filter_id(
            self.csp.as_ref().map(String::as_str),
            self.mask,
            self.filter.string_view().as_ref().map(|s| s.as_str()),
            self.hostname.as_ref().map(String::as_str),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref(),
        )
    }

    pub fn get_fuzzy_signature(&mut self) -> &Vec<Hash> {
        if self.fuzzy_signature.is_none() {
            if !self.is_fuzzy() {
                self.fuzzy_signature = Some(vec![]);
            } else {

            }
            self.fuzzy_signature = match &self.filter {
                FilterPart::Empty => Some(vec![]),
                FilterPart::Simple(filter) => Some(utils::create_fuzzy_signature(&filter)),
                FilterPart::AnyOf(filters) => {
                    Some(utils::create_combined_fuzzy_signature(&filters))
                }
            }
        }
        self.fuzzy_signature.as_ref().unwrap()
    }

    pub fn get_tokens(&self) -> Vec<Vec<Hash>> {
        let mut tokens: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);

        // If there is only one domain and no domain negation, we also use this
        // domain as a token.
        if self.opt_domains.is_some()
            && self.opt_not_domains.is_none()
            && self.opt_domains.as_ref().map(|d| d.len()) == Some(1)
        {
            if let Some(domains) = self.opt_domains.as_ref() {
                if let Some(domain) = domains.first() {
                    tokens.push(*domain)
                }
            }
        }

        // Get tokens from filter
        match &self.filter {
            FilterPart::Simple(f) => {
                if !self.is_complete_regex() {
                    let skip_last_token =
                        (self.is_plain() || self.is_regex()) && !self.is_right_anchor() && !self.is_fuzzy();
                    let skip_first_token = self.is_right_anchor();

                    let mut filter_tokens =
                        utils::tokenize_filter(&f, skip_first_token, skip_last_token);

                    tokens.append(&mut filter_tokens);
                }
            }
            FilterPart::AnyOf(_) => (), // across AnyOf set of filters no single token is guaranteed to match to a request
            _ => (),
        }

        // Append tokens from hostname, if any
        if let Some(hostname) = self.hostname.as_ref() {
            let mut hostname_tokens = utils::tokenize(&hostname);
            tokens.append(&mut hostname_tokens);
        }

        // If we got no tokens for the filter/hostname part, then we will dispatch
        // this filter in multiple buckets based on the domains option.
        if tokens.is_empty() && self.opt_domains.is_some() && self.opt_not_domains.is_none() {
            self.opt_domains
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|&d| vec![d])
                .collect()
        } else {
            // Add optional token for protocol
            if self.for_http() && !self.for_https() {
                tokens.push(utils::fast_hash("http"));
            } else if self.for_https() && !self.for_http() {
                tokens.push(utils::fast_hash("https"));
            }
            tokens.shrink_to_fit();
            vec![tokens]
        }
    }

    
    fn get_cpt_mask(&self) -> NetworkFilterMask {
        self.mask & NetworkFilterMask::FROM_ANY
    }
    
    pub fn is_fuzzy(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FUZZY_MATCH)
    }
    
    pub fn is_exception(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_EXCEPTION)
    }
    
    pub fn is_hostname_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR)
    }
    
    pub fn is_right_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_RIGHT_ANCHOR)
    }
    
    pub fn is_left_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR)
    }
    
    fn match_case(&self) -> bool {
        self.mask.contains(NetworkFilterMask::MATCH_CASE)
    }
    
    pub fn is_important(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_IMPORTANT)
    }
    
    pub fn is_redirect(&self) -> bool {
        self.redirect.is_some()
    }
    
    pub fn is_explicit_cancel(&self) -> bool {
        self.mask.contains(NetworkFilterMask::EXPLICIT_CANCEL)
    }
    
    pub fn is_badfilter(&self) -> bool {
        self.mask.contains(NetworkFilterMask::BAD_FILTER)
    }
    
    pub fn is_regex(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_REGEX)
    }
    
    pub fn is_complete_regex(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_COMPLETE_REGEX)
    }
    
    fn is_plain(&self) -> bool {
        !self.is_regex()
    }
    
    pub fn is_csp(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_CSP)
    }
    
    pub fn has_bug(&self) -> bool {
        self.bug.is_some()
    }
    
    fn cpt_any(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_ANY)
    }
    
    fn third_party(&self) -> bool {
        self.mask.contains(NetworkFilterMask::THIRD_PARTY)
    }
    
    fn first_party(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FIRST_PARTY)
    }
    
    fn for_http(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FROM_HTTP)
    }
    
    fn for_https(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FROM_HTTPS)
    }
}

pub trait NetworkMatchable {
    fn matches(&self, request: &request::Request) -> bool;
    fn get_regex(&self) -> Rc<CompiledRegex>;
}

impl NetworkMatchable for NetworkFilter {
    
    fn matches(&self, request: &request::Request) -> bool {
        check_options(&self, request) && check_pattern(&self, request)
    }

    // Lazily get the regex if the filter has one
    fn get_regex(&self) -> Rc<CompiledRegex> {
        if !self.is_regex() && !self.is_complete_regex() {
            return Rc::new(CompiledRegex::MatchAll);
        }
        // Create a new scope to contain the lifetime of the
        // dynamic borrow
        {
            let mut cache = self.regex.borrow_mut();
            if cache.is_some() {
                return cache.as_ref().unwrap().clone(); // Only clones the Arc, not the entire regex
            }
        
            let regex = compile_regex(
                &self.filter,
                self.is_right_anchor(),
                self.is_left_anchor(),
                self.is_complete_regex(),
            );

            *cache = Some(Rc::new(regex));
        }
        // Recursive call to return the just-cached value.
        // Note that if we had not let the previous borrow
        // of the cache fall out of scope then the subsequent
        // recursive borrow would cause a dynamic thread panic.
        // This is the major hazard of using `RefCell`.
        self.get_regex()
    }
}

// ---------------------------------------------------------------------------
// Filter parsing
// ---------------------------------------------------------------------------

fn compute_filter_id(
    csp: Option<&str>,
    mask: NetworkFilterMask,
    filter: Option<&str>,
    hostname: Option<&str>,
    opt_domains: Option<&Vec<Hash>>,
    opt_not_domains: Option<&Vec<Hash>>,
) -> Hash {
    let mut hash: Hash = (5408 * 33) ^ Hash::from(mask.bits);

    if let Some(s) = csp {
        let chars = s.chars();
        for c in chars {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    };

    if let Some(domains) = opt_domains {
        for d in domains {
            hash = hash.wrapping_mul(33) ^ d;
        }
    };

    if let Some(domains) = opt_not_domains {
        for d in domains {
            hash = hash.wrapping_mul(33) ^ d;
        }
    }

    if let Some(s) = filter {
        let chars = s.chars();
        for c in chars {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    }

    if let Some(s) = hostname {
        let chars = s.chars();
        for c in chars {
            hash = hash.wrapping_mul(33) ^ (c as Hash);
        }
    }

    hash
}

/**
 * Compiles a filter pattern to a regex. This is only performed *lazily* for
 * filters containing at least a * or ^ symbol. Because Regexes are expansive,
 * we try to convert some patterns to plain filters.
 */
#[allow(clippy::trivial_regex)]
pub fn compile_regex(
    filter: &FilterPart,
    is_right_anchor: bool,
    is_left_anchor: bool,
    is_complete_regex: bool,
) -> CompiledRegex {
    lazy_static! {
      // Escape special regex characters: |.$+?{}()[]\
      static ref SPECIAL_RE: Regex = Regex::new(r"([\|\.\$\+\?\{\}\(\)\[\]])").unwrap();
      // * can match anything
      static ref WILDCARD_RE: Regex = Regex::new(r"\*").unwrap();
      // ^ can match any separator or the end of the pattern
      static ref ANCHOR_RE: Regex = Regex::new(r"\^(.)").unwrap();
      // ^ can match any separator or the end of the pattern
      static ref ANCHOR_RE_EOL: Regex = Regex::new(r"\^$").unwrap();
    }

    let filters: Vec<String> = match filter {
        FilterPart::Empty => vec![],
        FilterPart::Simple(s) => vec![s.clone()],
        FilterPart::AnyOf(f) => f.clone(),
    };

    let mut escaped_patterns = Vec::with_capacity(filters.len());
    for filter_str in filters {
        // If any filter is empty, the entire set matches anything
        if filter_str.is_empty() {
            return CompiledRegex::MatchAll;
        }
        if is_complete_regex {
            // unescape unrecognised escaping sequences, otherwise a normal regex
            let unescaped = filter_str[1..filter_str.len() - 1]
                .replace("\\/", "/")
                .replace("\\:", ":");

            escaped_patterns.push(unescaped);
        } else {
            let repl = SPECIAL_RE.replace_all(&filter_str, "\\$1");
            let repl = WILDCARD_RE.replace_all(&repl, ".*");
            // in adblock rules, '^' is a separator.
            // The separator character is anything but a letter, a digit, or one of the following: _ - . %
            let repl = ANCHOR_RE.replace_all(&repl, "(?:[^\\w\\d\\._%-])$1");
            let repl = ANCHOR_RE_EOL.replace_all(&repl, "(?:[^\\w\\d\\._%-]|$)");

            // Should match start or end of url
            let left_anchor = if is_left_anchor { "^" } else { "" };
            let right_anchor = if is_right_anchor { "$" } else { "" };
            let filter = format!("{}{}{}", left_anchor, repl, right_anchor);

            escaped_patterns.push(filter);
        }
    }

    if escaped_patterns.is_empty() {
        CompiledRegex::MatchAll
    } else if escaped_patterns.len() == 1 {
        let pattern = &escaped_patterns[0];
        match Regex::new(&pattern) {
            Ok(compiled) => CompiledRegex::Compiled(compiled),
            Err(e) => {
                // println!("Regex parsing failed ({:?})", e);
                CompiledRegex::RegexParsingError(e)
            }
        }
    } else {
        match RegexSet::new(escaped_patterns) {
            Ok(compiled) => CompiledRegex::CompiledSet(compiled),
            Err(e) => CompiledRegex::RegexParsingError(e),
        }
    }
}

/**
 * Check if the sub-string contained between the indices start and end is a
 * regex filter (it contains a '*' or '^' char). Here we are limited by the
 * capability of javascript to check the presence of a pattern between two
 * indices (same for Regex...).
 * // TODO - we could use sticky regex here
 */
fn check_is_regex(filter: &str) -> bool {
    let start_index = filter.find('*');
    let separator_index = filter.find('^');
    start_index.is_some() || separator_index.is_some()
}

/**
 * Handle hostname anchored filters, given 'hostname' from ||hostname and
 * request's hostname, check if there is a match. This is tricky because filters
 * authors rely and different assumption. We can have prefix of suffix matches
 * of anchor.
 */
fn is_anchored_by_hostname(filter_hostname: &str, hostname: &str, wildcard_filter_hostname: bool) -> bool {
    let filter_hostname_len = filter_hostname.len();
    // Corner-case, if `filterHostname` is empty, then it's a match
    if filter_hostname_len == 0 {
        return true;
    }
    let hostname_len = hostname.len();

    
    if filter_hostname_len > hostname_len {
        // `filterHostname` cannot be longer than actual hostname
        false
    } else if filter_hostname_len == hostname_len {
        // If they have the same len(), they should be equal
        filter_hostname == hostname
    } else if let Some(match_index) = twoway::find_str(hostname, filter_hostname) { // Check if `filter_hostname` appears anywhere in `hostname`
        if match_index == 0 {
            // `filter_hostname` is a prefix of `hostname` and needs to match full a label.
            //
            // Examples (filter_hostname, hostname):
            //   * (foo, foo.com)
            //   * (sub.foo, sub.foo.com)
            wildcard_filter_hostname || filter_hostname.ends_with('.') || hostname[filter_hostname_len..].starts_with('.')
        } else if match_index == hostname_len - filter_hostname_len {
            // `filter_hostname` is a suffix of `hostname`.
            //
            // Examples (filter_hostname, hostname):
            //    * (foo.com, sub.foo.com)
            //    * (com, foo.com)
            filter_hostname.starts_with('.') || hostname[match_index - 1..].starts_with('.')
        } else {
            // `filter_hostname` is infix of `hostname` and needs match full labels
            (wildcard_filter_hostname || filter_hostname.ends_with('.') || hostname[filter_hostname_len..].starts_with('.'))
                && (filter_hostname.starts_with('.') || hostname[match_index - 1..].starts_with('.'))
        }
    }
    else {
        // No match
        false
    }
}

fn get_url_after_hostname<'a>(url: &'a str, hostname: &str) -> &'a str {
    let start = twoway::find_str(url, hostname).unwrap_or_else(|| url.len());
    &url[start + hostname.len()..]
}

// ---------------------------------------------------------------------------
// Filter matching
// ---------------------------------------------------------------------------

// pattern$fuzzy
fn check_pattern_fuzzy_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    filter
        .fuzzy_signature
        .as_ref()
        .map(|signature| {
            let request_signature = request.get_fuzzy_signature();

            if signature.len() > request_signature.len() {
                return false;
            }

            for filter_token in signature {
                // Find the occurrence of `c` in `request_signature`
                // Can assume fuzzy signatures are sorted
                if request_signature.binary_search(filter_token).is_err() {
                    return false;
                }
            }

            true
        })
        .unwrap_or(true) // corner case of rulle having fuzzy option but no filter
}

// pattern
fn check_pattern_plain_filter_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => twoway::find_str(&request.url, f).is_some(),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if twoway::find_str(&request.url, f).is_some() {
                    return true;
                }
            }
            false
        }
    }
}

// pattern|
fn check_pattern_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => request.url.ends_with(f),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if request.url.ends_with(f) {
                    return true;
                }
            }
            false
        }
    }
}

// |pattern
fn check_pattern_left_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => request.url.starts_with(f),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if request.url.starts_with(f) {
                    return true;
                }
            }
            false
        }
    }
}

// |pattern|
fn check_pattern_left_right_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => &request.url == f,
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if &request.url == f {
                    return true;
                }
            }
            false
        }
    }
}

// pattern*^
fn check_pattern_regex_filter_at(
    filter: &NetworkFilter,
    request: &request::Request,
    start_from: usize,
) -> bool {
    let regex = filter.get_regex();
    regex.is_match(&request.url[start_from..])
}

fn check_pattern_regex_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    check_pattern_regex_filter_at(filter, request, 0)
}

// ||pattern*^
fn check_pattern_hostname_anchor_regex_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                check_pattern_regex_filter_at(
                    filter,
                    request,
                    request.url.find(hostname).unwrap_or_default() + hostname.len(),
                )
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

// ||pattern|
fn check_pattern_hostname_right_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                match &filter.filter {
                    // In this specific case it means that the specified hostname should match
                    // at the end of the hostname of the request. This allows to prevent false
                    // positive like ||foo.bar which would match https://foo.bar.baz where
                    // ||foo.bar^ would not.
                    FilterPart::Empty => {
                        request.hostname.len() == hostname.len()        // if lengths are equal, hostname equality is implied by anchoring check
                            || request.hostname.ends_with(hostname)
                    }
                    _ => check_pattern_right_anchor_filter(&filter, request),
                }
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

// |||pattern|
fn check_pattern_hostname_left_right_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    // Since this is not a regex, the filter pattern must follow the hostname
    // with nothing in between. So we extract the part of the URL following
    // after hostname and will perform the matching on it.

    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Since it must follow immediatly after the hostname and be a suffix of
                    // the URL, we conclude that filter must be equal to the part of the
                    // url following the hostname.
                    FilterPart::Simple(f) => get_url_after_hostname(&request.url, hostname) == f,
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request.url, hostname);
                        for f in filters {
                            if url_after_hostname == f {
                                return true;
                            }
                        }
                        false
                    }
                }
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

// ||pattern + left-anchor => This means that a plain pattern needs to appear
// exactly after the hostname, with nothing in between.
fn check_pattern_hostname_left_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Since this is not a regex, the filter pattern must follow the hostname
                    // with nothing in between. So we extract the part of the URL following
                    // after hostname and will perform the matching on it.
                    FilterPart::Simple(f) => {
                        get_url_after_hostname(&request.url, hostname).starts_with(f)
                    }
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request.url, hostname);
                        for f in filters {
                            if url_after_hostname.starts_with(f) {
                                return true;
                            }
                        }
                        false
                    }
                }
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

// ||pattern
fn check_pattern_hostname_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Filter hostname does not necessarily have to be a full, proper hostname, part of it can be lumped together with the URL
                    FilterPart::Simple(f) => get_url_after_hostname(&request.url, hostname)
                        .find(f)
                        .is_some(),
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request.url, hostname);
                        for f in filters {
                            if url_after_hostname.find(f).is_some()
                            {
                                return true;
                            }
                        }
                        false
                    }
                }
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

// ||pattern$fuzzy
fn check_pattern_hostname_anchor_fuzzy_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(hostname, &request.hostname, filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX)) {
                check_pattern_fuzzy_filter(filter, request)
            } else {
                false
            }
        })
        .unwrap_or_else(|| unreachable!()) // no match if filter has no hostname - should be unreachable
}

/**
 * Specialize a network filter depending on its type. It allows for more
 * efficient matching function.
 */
fn check_pattern(filter: &NetworkFilter, request: &request::Request) -> bool {
    if filter.is_hostname_anchor() {
        if filter.is_regex() {
            check_pattern_hostname_anchor_regex_filter(filter, request)
        } else if filter.is_right_anchor() && filter.is_left_anchor() {
            check_pattern_hostname_left_right_anchor_filter(filter, request)
        } else if filter.is_right_anchor() {
            check_pattern_hostname_right_anchor_filter(filter, request)
        } else if filter.is_fuzzy() {
            check_pattern_hostname_anchor_fuzzy_filter(filter, request)
        } else if filter.is_left_anchor() {
            check_pattern_hostname_left_anchor_filter(filter, request)
        } else {
            check_pattern_hostname_anchor_filter(filter, request)
        }
    } else if filter.is_regex() || filter.is_complete_regex() {
        check_pattern_regex_filter(filter, request)
    } else if filter.is_left_anchor() && filter.is_right_anchor() {
        check_pattern_left_right_anchor_filter(filter, request)
    } else if filter.is_left_anchor() {
        check_pattern_left_anchor_filter(filter, request)
    } else if filter.is_right_anchor() {
        check_pattern_right_anchor_filter(filter, request)
    } else if filter.is_fuzzy() {
        check_pattern_fuzzy_filter(filter, request)
    } else {
        check_pattern_plain_filter_filter(filter, request)
    }
}

pub fn check_cpt_allowed(filter: &NetworkFilter, cpt: &request::RequestType) -> bool {
    match NetworkFilterMask::from(cpt) {
        NetworkFilterMask::UNMATCHED => filter.cpt_any(),
        mask => filter.mask.contains(mask),
    }
}

fn check_options(filter: &NetworkFilter, request: &request::Request) -> bool {
    // Bad filter never matches
    if filter.is_badfilter() {
        return false;
    }
    // We first discard requests based on type, protocol and party. This is really
    // cheap and should be done first.
    if !check_cpt_allowed(&filter, &request.request_type)
        || (request.is_https && !filter.for_https())
        || (request.is_http && !filter.for_http())
        || (!filter.first_party() && request.is_first_party == Some(true))
        || (!filter.third_party() && request.is_third_party == Some(true))
    {
        return false;
    }

    // Make sure that an exception with a bug ID can only apply to a request being
    // matched for a specific bug ID.
    if filter.bug.is_some() && filter.is_exception() && filter.bug != request.bug {
        return false;
    }

    
    // Source URL must be among these domains to match
    if let Some(included_domains) = filter.opt_domains.as_ref() {
        if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
            // If the union of included domains is recorded
            if let Some(included_domains_union) = filter.opt_domains_union {
                // If there isn't any source hash that matches the union, there's no match at all
                if source_hashes.iter().all(|h| h & included_domains_union != *h) {
                    return false
                }
            }
            if source_hashes.iter().all(|h| !utils::bin_lookup(&included_domains, *h)) {
                return false
            }
        }
    }

    if let Some(excluded_domains) = filter.opt_not_domains.as_ref() {
        if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
            // If the union of excluded domains is recorded
            if let Some(excluded_domains_union) = filter.opt_not_domains_union {
                // If there's any source hash that matches the union, check the actual values
                if source_hashes.iter().any(|h| (h & excluded_domains_union == *h) && utils::bin_lookup(&excluded_domains, *h)) {
                    return false
                }
            } else if source_hashes.iter().any(|h| utils::bin_lookup(&excluded_domains, *h)) {
                return false
            }
        }
    }

    true   
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct NetworkFilterBreakdown {
        filter: Option<String>,
        bug: Option<u32>,
        csp: Option<String>,
        hostname: Option<String>,
        opt_domains: Option<Vec<Hash>>,
        opt_not_domains: Option<Vec<Hash>>,
        redirect: Option<String>,

        // filter type
        is_fuzzy: bool,
        is_exception: bool,
        is_hostname_anchor: bool,
        is_right_anchor: bool,
        is_left_anchor: bool,
        is_regex: bool,
        is_csp: bool,
        is_plain: bool,
        is_important: bool,
        has_bug: bool,

        // Options
        first_party: bool,
        from_any: bool,
        from_font: bool,
        from_image: bool,
        from_media: bool,
        from_object: bool,
        from_other: bool,
        from_ping: bool,
        from_script: bool,
        from_stylesheet: bool,
        from_subdocument: bool,
        from_websocket: bool,
        from_xml_http_request: bool,
        match_case: bool,
        third_party: bool,
    }

    impl From<&NetworkFilter> for NetworkFilterBreakdown {
        fn from(filter: &NetworkFilter) -> NetworkFilterBreakdown {
            NetworkFilterBreakdown {
                filter: filter.filter.string_view(),
                bug: filter.bug.as_ref().cloned(),
                csp: filter.csp.as_ref().cloned(),
                hostname: filter.hostname.as_ref().cloned(),
                opt_domains: filter.opt_domains.as_ref().cloned(),
                opt_not_domains: filter.opt_not_domains.as_ref().cloned(),
                redirect: filter.redirect.as_ref().cloned(),

                // filter type
                is_fuzzy: filter.is_fuzzy(),
                is_exception: filter.is_exception(),
                is_hostname_anchor: filter.is_hostname_anchor(),
                is_right_anchor: filter.is_right_anchor(),
                is_left_anchor: filter.is_left_anchor(),
                is_regex: filter.is_regex(),
                is_csp: filter.is_csp(),
                is_plain: filter.is_plain(),
                is_important: filter.is_important(),
                has_bug: filter.has_bug(),

                // Options
                first_party: filter.first_party(),
                from_any: filter.cpt_any(),
                from_font: filter.mask.contains(NetworkFilterMask::FROM_FONT),
                from_image: filter.mask.contains(NetworkFilterMask::FROM_IMAGE),
                from_media: filter.mask.contains(NetworkFilterMask::FROM_MEDIA),
                from_object: filter.mask.contains(NetworkFilterMask::FROM_OBJECT),
                from_other: filter.mask.contains(NetworkFilterMask::FROM_OTHER),
                from_ping: filter.mask.contains(NetworkFilterMask::FROM_PING),
                from_script: filter.mask.contains(NetworkFilterMask::FROM_SCRIPT),
                from_stylesheet: filter.mask.contains(NetworkFilterMask::FROM_STYLESHEET),
                from_subdocument: filter.mask.contains(NetworkFilterMask::FROM_SUBDOCUMENT),
                from_websocket: filter.mask.contains(NetworkFilterMask::FROM_WEBSOCKET),
                from_xml_http_request: filter.mask.contains(NetworkFilterMask::FROM_XMLHTTPREQUEST),
                match_case: filter.match_case(),
                third_party: filter.third_party(),
            }
        }
    }

    fn default_network_filter_breakdown() -> NetworkFilterBreakdown {
        NetworkFilterBreakdown {
            filter: None,
            bug: None,
            csp: None,
            hostname: None,
            opt_domains: None,
            opt_not_domains: None,
            redirect: None,

            // filter type
            is_fuzzy: false,
            is_exception: false,
            is_hostname_anchor: false,
            is_right_anchor: false,
            is_left_anchor: false,
            is_regex: false,
            is_csp: false,
            is_plain: false,
            is_important: false,
            has_bug: false,

            // Options
            first_party: true,
            from_any: true,
            from_font: true,
            from_image: true,
            from_media: true,
            from_object: true,
            from_other: true,
            from_ping: true,
            from_script: true,
            from_stylesheet: true,
            from_subdocument: true,
            from_websocket: true,
            from_xml_http_request: true,
            match_case: false,
            third_party: true,
        }
    }

    #[test]
    // pattern
    fn parses_plain_pattern() {
        {
            let filter = NetworkFilter::parse("ads", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("ads"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("/ads/foo-", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("/ads/foo-"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("/ads/foo-$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("/ads/foo-"));
            defaults.is_plain = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("foo.com/ads$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_plain = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||pattern
    fn parses_hostname_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = None;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = None;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com/bar/baz$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||pattern|
    fn parses_hostname_right_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = None;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com|$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = None;
            defaults.is_plain = true;
            defaults.is_important = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com/bar/baz|$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com^bar/*baz|$important", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar/*baz"));
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |pattern
    fn parses_left_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com/bar/baz", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/bar/baz"));
            defaults.is_plain = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com^bar/*baz", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com^bar/*baz"));
            defaults.is_regex = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |pattern|
    fn parses_left_right_anchor_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com|", true).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com/bar|", true).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com/bar"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com*bar^|", true).unwrap();

            let mut defaults = default_network_filter_breakdown();
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_regex = true;
            defaults.is_right_anchor = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||regexp
    fn parses_hostname_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com*bar^", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com^bar*/baz^", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar*/baz^"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // ||regexp|
    fn parses_hostname_right_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("||foo.com*bar^|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("||foo.com^bar*/baz^|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("^bar*/baz^"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |regexp
    fn parses_hostname_left_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com*bar^", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com^bar*/baz^", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com^bar*/baz^"));
            defaults.is_left_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // |regexp|
    fn parses_hostname_left_right_anchor_regex_pattern() {
        {
            let filter = NetworkFilter::parse("|foo.com*bar^|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com*bar^"));
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
        {
            let filter = NetworkFilter::parse("|foo.com^bar*/baz^|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = None;
            defaults.filter = Some(String::from("foo.com^bar*/baz^"));
            defaults.is_left_anchor = true;
            defaults.is_right_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter))
        }
    }

    #[test]
    // @@pattern
    fn parses_exception_pattern() {
        {
            let filter = NetworkFilter::parse("@@ads", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("ads"));
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@||foo.com/ads", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("/ads"));
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@|foo.com/ads", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@|foo.com/ads|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@foo.com/ads|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("foo.com/ads"));
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("@@||foo.com/ads|", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.is_exception = true;
            defaults.filter = Some(String::from("/ads"));
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_left_anchor = true;
            defaults.is_plain = true;
            defaults.is_right_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
    }

    // Options

    #[test]
    fn accepts_any_content_type() {
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_any = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("||foo.com$first-party", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_any = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.first_party = true;
            defaults.third_party = false;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("||foo.com$third-party", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_any = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.first_party = false;
            defaults.third_party = true;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter = NetworkFilter::parse("||foo.com$domain=test.com", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_any = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.opt_domains = Some(vec![utils::fast_hash("test.com")]);

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
        {
            let filter =
                NetworkFilter::parse("||foo.com$domain=test.com,match-case", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.from_any = true;
            defaults.hostname = Some(String::from("foo.com"));
            defaults.is_hostname_anchor = true;
            defaults.is_plain = true;
            defaults.opt_domains = Some(vec![utils::fast_hash("test.com")]);
            defaults.match_case = true;

            assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
        }
    }

    #[test]
    fn parses_important() {
        {
            let filter = NetworkFilter::parse("||foo.com$important", true).unwrap();
            assert_eq!(filter.is_important(), true);
        }
        {
            // parses ~important
            let filter = NetworkFilter::parse("||foo.com$~important", true);
            assert_eq!(filter.err(), Some(FilterError::NegatedImportant));
        }
        {
            // defaults to false
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.is_important(), false);
        }
    }

    #[test]
    fn parses_csp() {
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.csp, None);
        }
        {
            // parses simple CSP
            let filter = NetworkFilter::parse(r#"||foo.com$csp=self bar """#, true).unwrap();
            assert_eq!(filter.is_csp(), true);
            assert_eq!(filter.csp, Some(String::from(r#"self bar """#)));
        }
        {
            // parses empty CSP
            let filter = NetworkFilter::parse("||foo.com$csp", true).unwrap();
            assert_eq!(filter.is_csp(), true);
            assert_eq!(filter.csp, None);
        }
        {
            // parses csp mixed with other options
            let filter =
                NetworkFilter::parse(r#"||foo.com$domain=foo|bar,csp=self bar "",image"#, true)
                    .unwrap();
            assert_eq!(filter.is_csp(), true);
            assert_eq!(filter.mask.contains(NetworkFilterMask::FROM_IMAGE), true);
            assert_eq!(filter.csp, Some(String::from(r#"self bar """#)));
        }
    }

    #[test]
    fn parses_domain() {
        // parses domain
        {
            let filter = NetworkFilter::parse("||foo.com$domain=bar.com", true).unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("bar.com")]));
            assert_eq!(filter.opt_not_domains, None);
        }
        {
            let filter = NetworkFilter::parse("||foo.com$domain=bar.com|baz.com", true).unwrap();
            let mut domains = vec![utils::fast_hash("bar.com"), utils::fast_hash("baz.com")];
            domains.sort_unstable();
            assert_eq!(filter.opt_domains, Some(domains));
            assert_eq!(filter.opt_not_domains, None);
        }

        // parses ~domain
        {
            let filter = NetworkFilter::parse("||foo.com$domain=~bar.com", true).unwrap();
            assert_eq!(filter.opt_domains, None);
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("bar.com")])
            );
        }
        {
            let filter = NetworkFilter::parse("||foo.com$domain=~bar.com|~baz.com", true).unwrap();
            assert_eq!(filter.opt_domains, None);
            let mut domains = vec![utils::fast_hash("bar.com"), utils::fast_hash("baz.com")];
            domains.sort_unstable();
            assert_eq!(filter.opt_not_domains, Some(domains));
        }
        // parses domain and ~domain
        {
            let filter = NetworkFilter::parse("||foo.com$domain=~bar.com|baz.com", true).unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("baz.com")]));
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("bar.com")])
            );
        }
        {
            let filter = NetworkFilter::parse("||foo.com$domain=bar.com|~baz.com", true).unwrap();
            assert_eq!(filter.opt_domains, Some(vec![utils::fast_hash("bar.com")]));
            assert_eq!(
                filter.opt_not_domains,
                Some(vec![utils::fast_hash("baz.com")])
            );
        }
        {
            let filter = NetworkFilter::parse("||foo.com$domain=foo|~bar|baz", true).unwrap();
            let mut domains = vec![utils::fast_hash("foo"), utils::fast_hash("baz")];
            domains.sort();
            assert_eq!(filter.opt_domains, Some(domains));
            assert_eq!(filter.opt_not_domains, Some(vec![utils::fast_hash("bar")]));
        }
        // defaults to no constraint
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.opt_domains, None);
            assert_eq!(filter.opt_not_domains, None);
        }
    }

    #[test]
    fn parses_redirects() {
        // parses redirect
        {
            let filter = NetworkFilter::parse("||foo.com$redirect=bar.js", true).unwrap();
            assert_eq!(filter.redirect, Some(String::from("bar.js")));
        }
        {
            let filter = NetworkFilter::parse("$redirect=bar.js", true).unwrap();
            assert_eq!(filter.redirect, Some(String::from("bar.js")));
        }
        // parses ~redirect
        {
            // ~redirect is not a valid option
            let filter = NetworkFilter::parse("||foo.com$~redirect", true);
            assert_eq!(filter.err(), Some(FilterError::NegatedRedirection));
        }
        // parses redirect without a value
        {
            // Not valid
            let filter = NetworkFilter::parse("||foo.com$redirect", true);
            assert_eq!(filter.err(), Some(FilterError::EmptyRedirection));
        }
        {
            let filter = NetworkFilter::parse("||foo.com$redirect=", true);
            assert_eq!(filter.err(), Some(FilterError::EmptyRedirection))
        }
        // defaults to false
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.redirect, None);
        }
    }

    #[test]
    fn parses_match_case() {
        // parses match-case
        {
            let filter = NetworkFilter::parse("||foo.com$match-case", true).unwrap();
            assert_eq!(filter.match_case(), true);
        }
        {
            let filter = NetworkFilter::parse("||foo.com$image,match-case", true).unwrap();
            assert_eq!(filter.match_case(), true);
        }
        {
            let filter = NetworkFilter::parse("||foo.com$media,match-case,image", true).unwrap();
            assert_eq!(filter.match_case(), true);
        }

        // parses ~match-case
        {
            // ~match-case is not supported
            let filter = NetworkFilter::parse("||foo.com$~match-case", true);
            assert_eq!(filter.err(), Some(FilterError::NegatedOptionMatchCase));
        }

        // defaults to false
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.match_case(), false)
        }
    }

    #[test]
    fn parses_first_party() {
        // parses first-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$first-party", true)
                .unwrap()
                .first_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com$first-party", true)
                .unwrap()
                .first_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com|$first-party", true)
                .unwrap()
                .first_party(),
            true
        );
        // parses ~first-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$~first-party", true)
                .unwrap()
                .first_party(),
            false
        );
        assert_eq!(
            NetworkFilter::parse("||foo.com$first-party,~first-party", true)
                .unwrap()
                .first_party(),
            false
        );
        // defaults to true
        assert_eq!(
            NetworkFilter::parse("||foo.com", true)
                .unwrap()
                .first_party(),
            true
        );
    }

    #[test]
    fn parses_third_party() {
        // parses third-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$third-party", true)
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com$third-party", true)
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("@@||foo.com|$third-party", true)
                .unwrap()
                .third_party(),
            true
        );
        assert_eq!(
            NetworkFilter::parse("||foo.com$~first-party", true)
                .unwrap()
                .third_party(),
            true
        );
        // parses ~third-party
        assert_eq!(
            NetworkFilter::parse("||foo.com$~third-party", true)
                .unwrap()
                .third_party(),
            false
        );
        assert_eq!(
            NetworkFilter::parse("||foo.com$first-party,~third-party", true)
                .unwrap()
                .third_party(),
            false
        );
        // defaults to true
        assert_eq!(
            NetworkFilter::parse("||foo.com", true)
                .unwrap()
                .third_party(),
            true
        );
    }

    #[test]
    fn parses_bug() {
        // parses bug
        {
            let filter = NetworkFilter::parse("||foo.com$bug=42", true).unwrap();
            assert_eq!(filter.has_bug(), true);
            assert_eq!(filter.bug, Some(42));
        }
        {
            let filter = NetworkFilter::parse("@@||foo.com$bug=1337", true).unwrap();
            assert_eq!(filter.is_exception(), true);
            assert_eq!(filter.has_bug(), true);
            assert_eq!(filter.bug, Some(1337));
        }
        {
            let filter = NetworkFilter::parse("@@||foo.com|$bug=11111", true).unwrap();
            assert_eq!(filter.is_exception(), true);
            assert_eq!(filter.has_bug(), true);
            assert_eq!(filter.bug, Some(11111));
        }
        {
            let filter = NetworkFilter::parse("@@$bug=11111", true).unwrap();
            assert_eq!(filter.is_exception(), true);
            assert_eq!(filter.has_bug(), true);
            assert_eq!(filter.bug, Some(11111));
        }

        // defaults to undefined
        {
            let filter = NetworkFilter::parse("||foo.com", true).unwrap();
            assert_eq!(filter.has_bug(), false);
            assert_eq!(filter.bug, None);
        }
    }

    #[test]
    fn handles_unsupported_options() {
        let options = vec![
            "genericblock",
            "generichide",
            "inline-script",
            "popunder",
            "popup",
            "woot",
        ];

        for option in options {
            let filter = NetworkFilter::parse(&format!("||foo.com${}", option), true);
            assert!(filter.err().is_some());
        }
    }

    #[test]
    fn handles_content_type_options() {
        let options = vec![
            "font",
            "image",
            "media",
            "object",
            "object-subrequest",
            "other",
            "ping",
            "script",
            "stylesheet",
            "subdocument",
            "websocket",
            "xmlhttprequest",
            "xhr",
        ];

        fn set_all_options(breakdown: &mut NetworkFilterBreakdown, value: bool) {
            breakdown.from_font = value;
            breakdown.from_image = value;
            breakdown.from_media = value;
            breakdown.from_object = value;
            breakdown.from_other = value;
            breakdown.from_ping = value;
            breakdown.from_script = value;
            breakdown.from_stylesheet = value;
            breakdown.from_subdocument = value;
            breakdown.from_websocket = value;
            breakdown.from_xml_http_request = value;
        }

        fn set_option(option: &str, breakdown: &mut NetworkFilterBreakdown, value: bool) {
            match option {
                "font" => breakdown.from_font = value,
                "image" => breakdown.from_image = value,
                "media" => breakdown.from_media = value,
                "object" => breakdown.from_object = value,
                "object-subrequest" => breakdown.from_object = value,
                "other" => breakdown.from_other = value,
                "ping" => breakdown.from_ping = value,
                "script" => breakdown.from_script = value,
                "stylesheet" => breakdown.from_stylesheet = value,
                "subdocument" => breakdown.from_subdocument = value,
                "websocket" => breakdown.from_websocket = value,
                "xmlhttprequest" => breakdown.from_xml_http_request = value,
                "xhr" => breakdown.from_xml_http_request = value,
                _ => unreachable!(),
            }
        }

        for option in options {
            // positive
            {
                let filter = NetworkFilter::parse(&format!("||foo.com${}", option), true).unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = false;
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter =
                    NetworkFilter::parse(&format!("||foo.com$object,{}", option), true).unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = false;
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                set_option("object", &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter =
                    NetworkFilter::parse(&format!("||foo.com$domain=bar.com,{}", option), true)
                        .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = false;
                defaults.opt_domains = Some(vec![utils::fast_hash("bar.com")]);
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            // negative
            {
                let filter = NetworkFilter::parse(&format!("||foo.com$~{}", option), true).unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = false;
                set_all_options(&mut defaults, true);
                set_option(&option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }

            {
                let filter =
                    NetworkFilter::parse(&format!("||foo.com${},~{}", option, option), true)
                        .unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = false;
                set_all_options(&mut defaults, false);
                set_option(&option, &mut defaults, false);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }
            // default - positive
            {
                let filter = NetworkFilter::parse(&format!("||foo.com"), true).unwrap();
                let mut defaults = default_network_filter_breakdown();
                defaults.hostname = Some(String::from("foo.com"));
                defaults.is_hostname_anchor = true;
                defaults.is_plain = true;
                defaults.from_any = true;
                set_all_options(&mut defaults, true);
                assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
            }
        }
    }

    use bincode::{deserialize, serialize};

    #[test]
    fn binary_serialization_works() {
        {
            let filter = NetworkFilter::parse("||foo.com/bar/baz$important", true).unwrap();

            let encoded: Vec<u8> = serialize(&filter).unwrap();
            let decoded: NetworkFilter = deserialize(&encoded[..]).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("/bar/baz"));
            defaults.is_plain = true;
            defaults.is_hostname_anchor = true;
            defaults.is_important = true;
            defaults.is_left_anchor = true;
            assert_eq!(defaults, NetworkFilterBreakdown::from(&decoded))
        }
        {
            let filter = NetworkFilter::parse("||foo.com*bar^", true).unwrap();
            let mut defaults = default_network_filter_breakdown();
            defaults.hostname = Some(String::from("foo.com"));
            defaults.filter = Some(String::from("bar^"));
            defaults.is_hostname_anchor = true;
            defaults.is_regex = true;
            defaults.is_plain = false;

            let encoded: Vec<u8> = serialize(&filter).unwrap();
            let decoded: NetworkFilter = deserialize(&encoded[..]).unwrap();

            assert_eq!(defaults, NetworkFilterBreakdown::from(&decoded));

            assert_eq!(decoded.get_regex().is_match("bar/"), true);
        }
    }

    #[test]
    fn parse_empty_host_anchor_exception() {
        let filter_parsed = NetworkFilter::parse("@@||$domain=auth.wi-fi.ru", true);
        assert!(filter_parsed.is_ok());

        let filter = filter_parsed.unwrap();

        let mut defaults = default_network_filter_breakdown();

        defaults.hostname = Some(String::from(""));
        defaults.is_hostname_anchor = true;
        defaults.is_exception = true;
        defaults.is_plain = true;
        defaults.from_any = true;
        defaults.opt_domains = Some(vec![utils::fast_hash("auth.wi-fi.ru")]);
        assert_eq!(defaults, NetworkFilterBreakdown::from(&filter));
    }

}

#[cfg(test)]
mod match_tests {
    use super::*;

    #[test]
    fn is_anchored_by_hostname_works() {
        // matches empty hostname
        assert_eq!(is_anchored_by_hostname("", "foo.com", false), true);

        // does not match when filter hostname is longer than hostname
        assert_eq!(is_anchored_by_hostname("bar.foo.com", "foo.com", false), false);
        assert_eq!(is_anchored_by_hostname("b", "", false), false);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.co", false), false);

        // does not match if there is not match
        assert_eq!(is_anchored_by_hostname("bar", "foo.com", false), false);

        // ## prefix match
        // matches exact match
        assert_eq!(is_anchored_by_hostname("", "", false), true);
        assert_eq!(is_anchored_by_hostname("f", "f", false), true);
        assert_eq!(is_anchored_by_hostname("foo", "foo", false), true);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com", ".com", false), true);
        assert_eq!(is_anchored_by_hostname("com.", "com.", false), true);

        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname("foo.", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".foo", ".foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".foo.", ".foo.com", false), true);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname("foo.com.", "foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname(".foo.com.", ".foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname(".foo.com", ".foo.com", false), true);

        assert_eq!(is_anchored_by_hostname("foo.bar", "foo.bar.com", false), true);
        assert_eq!(is_anchored_by_hostname("foo.bar.", "foo.bar.com", false), true);

        // does not match partial prefix
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foobar.com", false), false);
        assert_eq!(is_anchored_by_hostname("fo", "foo.com", false), false);
        assert_eq!(is_anchored_by_hostname(".foo", "foobar.com", false), false);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.bar", "foo.barbaz.com", false), false);
        assert_eq!(
            is_anchored_by_hostname(".foo.bar", ".foo.barbaz.com", false),
            false
        );

        // ## suffix match
        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com", "foo.com", false), true);
        assert_eq!(is_anchored_by_hostname(".com.", "foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname("com.", "foo.com.", false), true);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.com.", ".foo.com.", false), true);
        assert_eq!(is_anchored_by_hostname("foo.com", ".foo.com", false), true);

        // does not match partial
        // Single label
        assert_eq!(is_anchored_by_hostname("om", "foo.com", false), false);
        assert_eq!(is_anchored_by_hostname("com", "foocom", false), false);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.bar.com", "baz.bar.com", false), false);
        assert_eq!(is_anchored_by_hostname("fo.bar.com", "foo.bar.com", false), false);
        assert_eq!(is_anchored_by_hostname(".fo.bar.com", "foo.bar.com", false), false);
        assert_eq!(is_anchored_by_hostname("bar.com", "foobar.com", false), false);
        assert_eq!(is_anchored_by_hostname(".bar.com", "foobar.com", false), false);

        // ## infix match
        // matches partial
        assert_eq!(is_anchored_by_hostname("bar", "foo.bar.com", false), true);
        assert_eq!(is_anchored_by_hostname("bar.", "foo.bar.com", false), true);
        assert_eq!(is_anchored_by_hostname(".bar.", "foo.bar.com", false), true);
    }

    fn filter_match_url(filter: &str, url: &str, matching: bool) {
        let network_filter = NetworkFilter::parse(filter, true).unwrap();
        let request = request::Request::from_url(url).unwrap();

        assert!(
            network_filter.matches(&request) == matching,
            "Expected match={} for {} on {}",
            matching,
            filter,
            url
        );
    }

    #[test]
    // pattern
    fn check_pattern_plain_filter_filter_works() {
        filter_match_url("foo", "https://bar.com/foo", true);
        filter_match_url("foo", "https://bar.com/baz/foo", true);
        filter_match_url("foo", "https://bar.com/q=foo/baz", true);
        filter_match_url("foo", "https://foo.com", true);
        filter_match_url("-foo-", "https://bar.com/baz/42-foo-q", true);
        filter_match_url("&fo.o=+_-", "https://bar.com?baz=42&fo.o=+_-", true);
        filter_match_url("foo/bar/baz", "https://bar.com/foo/bar/baz", true);
        filter_match_url("com/bar/baz", "https://bar.com/bar/baz", true);
        filter_match_url("https://bar.com/bar/baz", "https://bar.com/bar/baz", true);
    }

    #[test]
    // pattern$fuzzy
    fn check_pattern_fuzzy_filter_works() {
        filter_match_url("f$fuzzy", "https://bar.com/f", true);
        filter_match_url("foo$fuzzy", "https://bar.com/foo", true);
        filter_match_url("foo$fuzzy", "https://bar.com/foo/baz", true);
        filter_match_url("foo/bar$fuzzy", "https://bar.com/foo/baz", true);
        filter_match_url("foo bar$fuzzy", "https://bar.com/foo/baz", true);
        filter_match_url("foo bar baz$fuzzy", "http://bar.foo.baz", true);

        filter_match_url("foo bar baz 42$fuzzy", "http://bar.foo.baz", false);

        // Fast-path for when pattern is longer than the URL
        filter_match_url("foo bar baz 42 43$fuzzy", "http://bar.foo.baz", false);

        // No fuzzy signature, matches every URL?
        filter_match_url("+$fuzzy", "http://bar.foo.baz", true);
        filter_match_url("$fuzzy", "http://bar.foo.baz", true);
    }

    #[test]
    // ||pattern
    fn check_pattern_hostname_anchor_filter_works() {
        filter_match_url("||foo.com", "https://foo.com/bar", true);
        filter_match_url("||foo.com/bar", "https://foo.com/bar", true);
        filter_match_url("||foo", "https://foo.com/bar", true);
        filter_match_url("||foo", "https://baz.foo.com/bar", true);
        filter_match_url("||foo", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz.", "https://foo.baz.com/bar", true);

        filter_match_url("||foo.baz.com^", "https://foo.baz.com/bar", true);
        filter_match_url("||foo.baz^", "https://foo.baz.com/bar", false);

        filter_match_url("||foo", "https://baz.com", false);
        filter_match_url("||foo", "https://foo-bar.baz.com/bar", false);
        filter_match_url("||foo.com", "https://foo.de", false);
        filter_match_url("||foo.com", "https://bar.foo.de", false);
    }

    #[test]
    // ||pattern$fuzzy
    fn check_pattern_hostname_anchor_fuzzy_filter_works() {
        let network_filter = NetworkFilter::parse("||bar.foo/baz$fuzzy", true).unwrap();
        let request = request::Request::from_url("http://bar.foo/baz").unwrap();
        assert_eq!(network_filter.matches(&request), true);
        // Same result when fuzzy signature is cached
        assert_eq!(network_filter.matches(&request), true);

        filter_match_url("||bar.foo/baz$fuzzy", "http://bar.foo/id/baz", true);
        filter_match_url("||bar.foo/baz$fuzzy", "http://bar.foo?id=42&baz=1", true);
        filter_match_url("||foo.com/id bar$fuzzy", "http://foo.com?bar&id=42", true);

        filter_match_url("||bar.com/id bar$fuzzy", "http://foo.com?bar&id=42", false);
        filter_match_url(
            "||bar.com/id bar baz foo 42 id$fuzzy",
            "http://foo.com?bar&id=42",
            false,
        );
    }

    #[test]
    // ||pattern|
    fn check_pattern_hostname_right_anchor_filter_works() {
        filter_match_url("||foo.com|", "https://foo.com", true);
        filter_match_url("||foo.com/bar|", "https://foo.com/bar", true);

        filter_match_url("||foo.com/bar|", "https://foo.com/bar/baz", false);
        filter_match_url("||foo.com/bar|", "https://foo.com/", false);
        filter_match_url("||bar.com/bar|", "https://foo.com/", false);
    }

    #[test]
    // pattern|
    fn check_pattern_right_anchor_filter_works() {
        filter_match_url("foo.com", "https://foo.com", true);
        filter_match_url("foo|", "https://bar.com/foo", true);
        filter_match_url("foo|", "https://bar.com/foo/", false);
        filter_match_url("foo|", "https://bar.com/foo/baz", false);
    }

    #[test]
    // |pattern
    fn check_pattern_left_anchor_filter_works() {
        filter_match_url("|http", "http://foo.com", true);
        filter_match_url("|http", "https://foo.com", true);
        filter_match_url("|https://", "https://foo.com", true);

        filter_match_url("https", "http://foo.com", false);
    }

    #[test]
    // |pattern|
    fn check_pattern_left_right_anchor_filter_works() {
        filter_match_url("|https://foo.com|", "https://foo.com", true);
    }

    #[test]
    // ||pattern + left-anchor
    fn check_pattern_hostname_left_anchor_filter_works() {
        filter_match_url("||foo.com^test", "https://foo.com/test", true);
        filter_match_url("||foo.com/test", "https://foo.com/test", true);
        filter_match_url("||foo.com^test", "https://foo.com/tes", false);
        filter_match_url("||foo.com/test", "https://foo.com/tes", false);

        filter_match_url("||foo.com^", "https://foo.com/test", true);

        filter_match_url("||foo.com/test*bar", "https://foo.com/testbar", true);
        filter_match_url("||foo.com^test*bar", "https://foo.com/testbar", true);
    }

    #[test]
    // ||hostname^*/pattern
    fn check_pattern_hostname_anchor_regex_filter_works() {
        filter_match_url("||foo.com^*/bar", "https://foo.com/bar", false);
        filter_match_url("||com^*/bar", "https://foo.com/bar", false);
        filter_match_url("||foo^*/bar", "https://foo.com/bar", false);

        // @see https://github.com/cliqz-oss/adblocker/issues/29
        filter_match_url("||foo.co^aaa/", "https://bar.foo.com/bbb/aaa/", false);
        filter_match_url("||foo.com^aaa/", "https://bar.foo.com/bbb/aaa/", false);

        filter_match_url("||com*^bar", "https://foo.com/bar", true);
        filter_match_url("||foo.com^bar", "https://foo.com/bar", true);
        filter_match_url("||com^bar", "https://foo.com/bar", true);
        filter_match_url("||foo*^bar", "https://foo.com/bar", true);
        filter_match_url("||foo*/bar", "https://foo.com/bar", true);
        filter_match_url("||foo*com/bar", "https://foo.com/bar", true);
        filter_match_url("||foo2*com/bar", "https://foo2.com/bar", true);
        filter_match_url("||foo*com*/bar", "https://foo.com/bar", true);
        filter_match_url("||foo*com*^bar", "https://foo.com/bar", true);
        filter_match_url("||*foo*com*^bar", "https://foo.com/bar", true);
        filter_match_url("||*/bar", "https://foo.com/bar", true);
        filter_match_url("||*^bar", "https://foo.com/bar", true);
        filter_match_url("||*com/bar", "https://foo.com/bar", true);
        filter_match_url("||*.com/bar", "https://foo.com/bar", true);
        filter_match_url("||*foo.com/bar", "https://foo.com/bar", true);
        filter_match_url("||*com/bar", "https://foo.com/bar", true);
        filter_match_url("||*com*/bar", "https://foo.com/bar", true);
        filter_match_url("||*com*^bar", "https://foo.com/bar", true);
    }

    #[test]
    fn check_pattern_hostname_anchor_regex_filter_works_realisitic() {
        filter_match_url("||vimeo.com^*?type=", "https://vimeo.com/ablincoln/fatal_attraction?type=pageview&target=%2F193641463", true);
    }

    #[test]
    fn check_pattern_hostname_left_right_anchor_regex_filter_works() {
        filter_match_url("||geo*.hltv.org^", "https://geo2.hltv.org/rekl13.php", true);
        filter_match_url(
            "||www*.swatchseries.to^",
            "https://www1.swatchseries.to/sw.js",
            true,
        );
        filter_match_url("||imp*.tradedoubler.com^", "https://impde.tradedoubler.com/imp?type(js)g(22608602)a(1725113)epi(30148500144427100033372010772028)preurl(https://pixel.mathtag.com/event/js?mt_id=1160537&mt_adid=166882&mt_exem=&mt_excl=&v1=&v2=&v3=&s1=&s2=&s3=&mt_nsync=1&redirect=https%3A%2F%2Fad28.ad-srv.net%2Fc%2Fczqwm6dm6kagr2j%3Ftprde%3D)768489806", true);
    }

    #[test]
    fn check_pattern_exception_works() {
        {
            let filter = "@@||fastly.net/ad2/$image,script,xmlhttprequest";
            let url = "https://0914.global.ssl.fastly.net/ad2/script/x.js?cb=1549980040838";
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let request = request::Request::from_urls(
                url,
                "https://www.gamespot.com/metro-exodus/",
                "script",
            )
            .unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            let filter = "@@||swatchseries.to/public/js/edit-show.js$script,domain=swatchseries.to";
            let url = "https://www1.swatchseries.to/public/js/edit-show.js";
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let request = request::Request::from_urls(
                url,
                "https://www1.swatchseries.to/serie/roswell_new_mexico",
                "script",
            )
            .unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    // options
    fn check_options_works() {
        // cpt test
        {
            let network_filter = NetworkFilter::parse("||foo$image", true).unwrap();
            let request = request::Request::from_urls("https://foo.com/bar", "", "image").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$image", true).unwrap();
            let request = request::Request::from_urls("https://foo.com/bar", "", "script").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$~image", true).unwrap();
            let request = request::Request::from_urls("https://foo.com/bar", "", "script").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }

        // ~third-party
        {
            let network_filter = NetworkFilter::parse("||foo$~third-party", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://baz.foo.com", "")
                    .unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$~third-party", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://baz.bar.com", "")
                    .unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // ~first-party
        {
            let network_filter = NetworkFilter::parse("||foo$~first-party", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://baz.bar.com", "")
                    .unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$~first-party", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://baz.foo.com", "")
                    .unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // opt-domain
        {
            let network_filter = NetworkFilter::parse("||foo$domain=foo.com", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$domain=foo.com", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }

        // opt-not-domain
        {
            let network_filter = NetworkFilter::parse("||foo$domain=~bar.com", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://foo.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), true);
        }
        {
            let network_filter = NetworkFilter::parse("||foo$domain=~bar.com", true).unwrap();
            let request =
                request::Request::from_urls("https://foo.com/bar", "http://bar.com", "").unwrap();
            assert_eq!(check_options(&network_filter, &request), false);
        }
    }

    #[test]
    fn check_domain_option_subsetting_works() {
        {
            let network_filter = NetworkFilter::parse("adv$domain=example.com|~foo.example.com", true).unwrap();
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://subfoo.foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://bar.example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://anotherexample.com", "").unwrap()) == false);
        }
        {
            let network_filter = NetworkFilter::parse("adv$domain=~example.com|~foo.example.com", true).unwrap();
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://subfoo.foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://bar.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://anotherexample.com", "").unwrap()) == true);
        }
        {
            let network_filter = NetworkFilter::parse("adv$domain=example.com|foo.example.com", true).unwrap();
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://foo.example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://subfoo.foo.example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://bar.example.com", "").unwrap()) == true);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://anotherexample.com", "").unwrap()) == false);
        }
        {
            let network_filter = NetworkFilter::parse("adv$domain=~example.com|foo.example.com", true).unwrap();
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://subfoo.foo.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://bar.example.com", "").unwrap()) == false);
            assert!(network_filter.matches(&request::Request::from_urls("http://example.net/adv", "http://anotherexample.com", "").unwrap()) == false);
        }
        
    }

    #[test]
    fn check_unicode_handled() {
        filter_match_url(
            "||firstrowsports.li/frame/",
            "https://firstrowsports.li/frame/bar",
            true,
        );
        filter_match_url(
            "||fırstrowsports.eu/pu/",
            "https://fırstrowsports.eu/pu/foo",
            true,
        );
        filter_match_url(
            "||fırstrowsports.eu/pu/",
            "https://xn--frstrowsports-39b.eu/pu/foo",
            true,
        );

        filter_match_url("||atđhe.net/pu/", "https://atđhe.net/pu/foo", true);
        filter_match_url("||atđhe.net/pu/", "https://xn--athe-1ua.net/pu/foo", true);
    }

    #[test]
    fn check_regex_escaping_handled() {
        // A few rules that are not correctly escaped for rust Regex
        {
            // regex escaping "\/" unrecognised
            let filter =
                r#"/^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/$domain=123movies.com|1337x.to"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://bit.ly/bar/";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            // regex escaping "\:" unrecognised
            let filter = r#"/\:\/\/data.*\.com\/[a-zA-Z0-9]{30,}/$third-party,xmlhttprequest"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "xmlhttprequest").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        //
        {
            let filter = r#"/\.(accountant|bid|click|club|com|cricket|date|download|faith|link|loan|lol|men|online|party|racing|review|science|site|space|stream|top|trade|webcam|website|win|xyz|com)\/(([0-9]{2,9})(\.|\/)(css|\?)?)$/$script,stylesheet,third-party,xmlhttprequest"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://hello.club/123.css";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "stylesheet").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    #[ignore] // Not going to handle lookaround regexes
    fn check_lookaround_regex_handled() {
        {
            let filter = r#"/^https?:\/\/([0-9a-z\-]+\.)?(9anime|animeland|animenova|animeplus|animetoon|animewow|gamestorrent|goodanime|gogoanime|igg-games|kimcartoon|memecenter|readcomiconline|toonget|toonova|watchcartoononline)\.[a-z]{2,4}\/(?!([Ee]xternal|[Ii]mages|[Ss]cripts|[Uu]ploads|ac|ajax|assets|combined|content|cov|cover|(img\/bg)|(img\/icon)|inc|jwplayer|player|playlist-cat-rss|static|thumbs|wp-content|wp-includes)\/)(.*)/$image,other,script,~third-party,xmlhttprequest,domain=~animeland.hu"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let regex = Rc::try_unwrap(network_filter.get_regex()).unwrap();
            assert!(
                matches!(regex, CompiledRegex::Compiled(_)),
                "Generated incorrect regex: {:?}",
                regex
            );
            let url = "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "script").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

    #[test]
    fn check_empty_host_anchor_matches() {
        {
            let filter = "||$domain=auth.wi-fi.ru";
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://example.com/ad.js";
            let source = "http://auth.wi-fi.ru";
            let request = request::Request::from_urls(url, source, "script").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
        {
            let filter = "@@||$domain=auth.wi-fi.ru";
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://example.com/ad.js";
            let source = "http://auth.wi-fi.ru";
            let request = request::Request::from_urls(url, source, "script").unwrap();
            assert!(
                network_filter.matches(&request) == true,
                "Expected match for {} on {}",
                filter,
                url
            );
        }
    }

}
