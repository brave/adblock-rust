use idna;
use regex::Regex;
use std::fmt;
use std::cell::RefCell;

use crate::request;
use crate::utils;
use crate::utils::Hash;

pub const TOKENS_BUFFER_SIZE: usize = 200;

#[derive(Debug, PartialEq)]
pub enum FilterError {
    FilterParseError,
    BadFilter,
    NegatedImportant,
    NegatedOptionMatchCase,
    NegatedRedirection,
    EmptyRedirection,
    UnrecognisedOption,
    NoRegex,
    RegexParsingError(regex::Error),
    PunycodeError
}

bitflags! {
    struct NetworkFilterMask: u32 {
        const FROM_IMAGE = 1 << 0;
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

        // Kind of pattern
        const THIRD_PARTY = 1 << 16;
        const FIRST_PARTY = 1 << 17;
        const IS_REGEX = 1 << 18;
        const IS_LEFT_ANCHOR = 1 << 19;
        const IS_RIGHT_ANCHOR = 1 << 20;
        const IS_HOSTNAME_ANCHOR = 1 << 21;
        const IS_EXCEPTION = 1 << 22;
        const IS_CSP = 1 << 23;

        // "Other" network request types
        const UNMATCHED = 1 << 24;

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

        // Careful with checking for NONE - will always match
        const NONE = 0;
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
            request::RequestType::Other => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Script => NetworkFilterMask::FROM_SCRIPT,
            request::RequestType::Image => NetworkFilterMask::FROM_IMAGE,
            request::RequestType::Stylesheet => NetworkFilterMask::FROM_STYLESHEET,
            request::RequestType::Object => NetworkFilterMask::FROM_OBJECT,
            request::RequestType::Subdocument => NetworkFilterMask::FROM_SUBDOCUMENT,
            request::RequestType::Ping => NetworkFilterMask::FROM_PING,
            request::RequestType::Beacon => NetworkFilterMask::FROM_PING,
            request::RequestType::Xmlhttprequest => NetworkFilterMask::FROM_XMLHTTPREQUEST,
            request::RequestType::Font => NetworkFilterMask::FROM_FONT,
            request::RequestType::Media => NetworkFilterMask::FROM_MEDIA,
            request::RequestType::Websocket => NetworkFilterMask::FROM_WEBSOCKET,
            request::RequestType::Dtd => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Fetch => NetworkFilterMask::FROM_OTHER,
            request::RequestType::Xlst => NetworkFilterMask::FROM_OTHER,
            // TODO: check if the logic is actually correct, TS implementation mapping was non-exhaustive, following options added:
            request::RequestType::Document => NetworkFilterMask::UNMATCHED,
            request::RequestType::Csp => NetworkFilterMask::UNMATCHED,
        }
    }
}

pub struct NetworkFilter {
    mask: NetworkFilterMask,
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
    pub id: Hash,
    pub fuzzy_signature: Option<Vec<Hash>>,
    
    regex: RefCell<Option<Regex>>, // compiled lazily

    #[cfg(feature="full-domain-matching")] pub opt_domains_full: Option<Vec<String>>,
    #[cfg(feature="full-domain-matching")] pub opt_not_domains_full: Option<Vec<String>>
}

impl NetworkFilter {
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

        #[cfg(feature="full-domain-matching")] let mut opt_domains_full: Option<Vec<String>> = None;
        #[cfg(feature="full-domain-matching")] let mut opt_not_domains_full: Option<Vec<String>> = None;

        let mut redirect: Option<String> = None;
        let mut csp: Option<String> = None;
        let mut bug: Option<u32> = None;

        // Start parsing
        let mut filter_index_start: usize = 0;
        let mut filter_index_end: usize = line.len();

        // @@filter == Exception
        if utils::fast_starts_with(line, "@@") {
            filter_index_start += 2;
            mask.set(NetworkFilterMask::IS_EXCEPTION, true);
        }

        // filter$options == Options
        // ^     ^
        // |     |
        // |     optionsIndex
        // filterIndexStart
        let options_index: Option<usize> = line.rfind("$");

        if options_index.is_some() {
            // Parse options and set flags
            filter_index_end = options_index.unwrap();

            // --------------------------------------------------------------------- //
            // parseOptions
            // TODO: This could be implemented without string copy,
            // using indices, like in main parsing functions.
            let raw_options = &line[filter_index_end + 1..];
            let options = raw_options.split(',');
            for raw_option in options {
                // Check for negation: ~option
                let negation = utils::fast_starts_with(&raw_option, "~");
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

                        let mut valuesiter = option_values.iter();

                        while let Some(option_value) = valuesiter.next() {
                            if utils::fast_starts_with(option_value, "~") {
                                let domain = &option_value[1..];
                                let domain_hash = utils::fast_hash(&domain);
                                opt_not_domains_array.push(domain_hash);
                            } else {
                                let domain_hash = utils::fast_hash(&option_value);
                                opt_domains_array.push(domain_hash);
                            }
                        }

                        #[cfg(feature="full-domain-matching")]
                        {
                            let mut opt_domains_full_array: Vec<String> = vec![];
                            let mut opt_not_domains_full_array: Vec<String> = vec![];
                            let mut valuesiter = option_values.iter();
                            while let Some(option_value) = valuesiter.next() {
                                if utils::fast_starts_with(option_value, "~") {
                                    opt_not_domains_full_array.push(String::from(&option_value[1..]));
                                } else {
                                    opt_domains_full_array.push(String::from(&option_value[0..]));
                                }
                            }
                            if opt_domains_array.len() > 0 {
                                opt_domains_full_array.sort();
                                opt_domains_full = Some(opt_domains_full_array);
                            }
                            if opt_not_domains_array.len() > 0 {
                                opt_not_domains_full_array.sort();
                                opt_not_domains_full = Some(opt_not_domains_full_array);
                            }
                        }

                        if opt_domains_array.len() > 0 {
                            opt_domains_array.sort();
                            opt_domains = Some(opt_domains_array);
                        }
                        if opt_not_domains_array.len() > 0 {
                            opt_not_domains_array.sort();
                            opt_not_domains = Some(opt_not_domains_array);
                        }
                    }
                    // TODO - how to handle those, if we start in mask, then the id will
                    // differ from the other filter. We could keep original line. How do
                    // to eliminate thos efficiently? They will probably endup in the same
                    // bucket, so maybe we could do that on a per-bucket basis?
                    ("badfilter", _) => return Err(FilterError::BadFilter),
                    // Note: `negation` should always be `false` here.
                    ("important", true) => return Err(FilterError::NegatedImportant),
                    ("important", false) => mask.set(NetworkFilterMask::IS_IMPORTANT, true),
                    // Note: `negation` should always be `false` here.
                    ("match-case", true) => return Err(FilterError::NegatedOptionMatchCase),
                    ("match-case", false) => mask.set(NetworkFilterMask::MATCH_CASE, true),
                    // ~third-party means we should clear the flag
                    ("third-party", true) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("third-party", false) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    // ~first-party means we should clear the flag
                    ("first-party", true) => mask.set(NetworkFilterMask::FIRST_PARTY, false),
                    // first-party means ~third-party
                    ("first-party", false) => mask.set(NetworkFilterMask::THIRD_PARTY, false),
                    ("fuzzy", _) => mask.set(NetworkFilterMask::FUZZY_MATCH, true),
                    ("collapse", _) => {}
                    ("bug", _) => bug = value.parse::<u32>().ok(),
                    // Negation of redirection doesn't make sense
                    ("redirect", true) => return Err(FilterError::NegatedRedirection),
                    ("redirect", false) => {
                        // Ignore this filter if no redirection resource is specified
                        if value.len() == 0 {
                            return Err(FilterError::EmptyRedirection);
                        }

                        redirect = Some(String::from(value));
                    }
                    ("csp", _) => {
                        mask.set(NetworkFilterMask::IS_CSP, true);
                        if value.len() > 0 {
                            csp = Some(String::from(value));
                        }
                    }
                    (_, negation) => {
                        // Handle content type options separatly
                        let mut option_mask = NetworkFilterMask::NONE;
                        match option {
                            "image" => option_mask.set(NetworkFilterMask::FROM_IMAGE, true),
                            "media" => option_mask.set(NetworkFilterMask::FROM_MEDIA, true),
                            "object" => option_mask.set(NetworkFilterMask::FROM_OBJECT, true),
                            "object-subrequest" => {
                                option_mask.set(NetworkFilterMask::FROM_OBJECT, true)
                            }
                            "other" => option_mask.set(NetworkFilterMask::FROM_OTHER, true),
                            "ping" => option_mask.set(NetworkFilterMask::FROM_PING, true),
                            "beacon" => option_mask.set(NetworkFilterMask::FROM_PING, true),
                            "script" => option_mask.set(NetworkFilterMask::FROM_SCRIPT, true),
                            "stylesheet" => {
                                option_mask.set(NetworkFilterMask::FROM_STYLESHEET, true)
                            }
                            "subdocument" => {
                                option_mask.set(NetworkFilterMask::FROM_SUBDOCUMENT, true)
                            }
                            "xmlhttprequest" => {
                                option_mask.set(NetworkFilterMask::FROM_XMLHTTPREQUEST, true)
                            }
                            "xhr" => option_mask.set(NetworkFilterMask::FROM_XMLHTTPREQUEST, true),
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

        // Deal with hostname pattern
        if filter_index_end > 0 && utils::fast_starts_with_from(line, "|", filter_index_end - 1) {
            mask.set(NetworkFilterMask::IS_RIGHT_ANCHOR, true);
            filter_index_end -= 1;
        }

        if utils::fast_starts_with_from(line, "||", filter_index_start) {
            mask.set(NetworkFilterMask::IS_HOSTNAME_ANCHOR, true);
            filter_index_start += 2;
        } else if utils::fast_starts_with_from(line, "|", filter_index_start) {
            mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
            filter_index_start += 1;
        }

        let is_regex = check_is_regex(&line[filter_index_start..filter_index_end]);
        mask.set(NetworkFilterMask::IS_REGEX, is_regex);

        if mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR) {
            if is_regex {
                // Split at the first '/', '*' or '^' character to get the hostname
                // and then the pattern.
                // TODO - this could be made more efficient if we could match between two
                // indices. Once again, we have to do more work than is really needed.
                lazy_static! {
                    static ref SEPARATOR: Regex = Regex::new("[/^*]").unwrap();
                }
                let first_separator = SEPARATOR.find(line).unwrap().start();
                // NOTE: `first_separator` shall never be -1 here since `IS_REGEX` is true.
                // This means there must be at least an occurrence of `*` or `^`
                // somewhere.

                hostname = Some(String::from(&line[filter_index_start..first_separator]));
                filter_index_start = first_separator;

                // If the only symbol remaining for the selector is '^' then ignore it
                // but set the filter as right anchored since there should not be any
                // other label on the right
                if filter_index_end - filter_index_start == 1
                    && utils::fast_starts_with_from(&line, "^", filter_index_start)
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
            } else {
                // Look for next /
                let slash_index = &line[filter_index_start..].find('/');
                slash_index
                    .map(|i| {
                        hostname = Some(String::from(
                            &line[filter_index_start..filter_index_start + i],
                        ));
                        filter_index_start = filter_index_start + i;
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
            && utils::fast_starts_with_from(&line, "*", filter_index_end - 1)
        {
            filter_index_end -= 1;
        }

        // Remove leading '*' if the filter is not hostname anchored.
        if filter_index_end - filter_index_start > 0
            && utils::fast_starts_with_from(&line, "*", filter_index_start)
        {
            mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
            filter_index_start += 1;
        }

        // Transform filters on protocol (http, https, ws)
        if mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR) {
            if filter_index_end - filter_index_start == 5
                && utils::fast_starts_with_from(line, "ws://", filter_index_start)
            {
                mask.set(NetworkFilterMask::FROM_WEBSOCKET, true);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 7
                && utils::fast_starts_with_from(line, "http://", filter_index_start)
            {
                mask.set(NetworkFilterMask::FROM_HTTP, true);
                mask.set(NetworkFilterMask::FROM_HTTPS, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 8
                && utils::fast_starts_with_from(line, "https://", filter_index_start)
            {
                mask.set(NetworkFilterMask::FROM_HTTPS, true);
                mask.set(NetworkFilterMask::FROM_HTTP, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end - filter_index_start == 8
                && utils::fast_starts_with_from(line, "http*://", filter_index_start)
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
                    verify_dns_length: true
                };
                match idna::uts46::to_ascii(&lowercase, decode_flags) {
                    Ok(x) => hostname.push_str(&x),
                    Err(_) => return Err(FilterError::PunycodeError)
                }
            }
            Ok(hostname)
        });

        // TODO: eval impact - fuzzy signature gets compiled in original code lazily
        let maybe_fuzzy_signature = if mask.contains(NetworkFilterMask::FUZZY_MATCH) {
            filter.as_ref().map(|f| utils::create_fuzzy_signature(f))
        } else {
            None
        };

        Ok(NetworkFilter {
            bug: bug,
            csp: csp,
            filter: filter,
            hostname: hostname_decoded.map_or(Ok(None), |r| r.map(Some))?,
            mask: mask,
            opt_domains: opt_domains,
            opt_not_domains: opt_not_domains,
            raw_line: if debug {
                Some(String::from(line))
            } else {
                None
            },
            redirect: redirect,
            debug: debug,
            id: utils::fast_hash(&line),
            fuzzy_signature: maybe_fuzzy_signature,
            regex: RefCell::default(),
            #[cfg(feature="full-domain-matching")] opt_domains_full,
            #[cfg(feature="full-domain-matching")] opt_not_domains_full,
        })
    }

    pub fn matches(&self, request: &request::Request) -> bool {
        check_options(&self, request) && check_pattern(&self, request)
    }

    pub fn to_string(&self) -> String {
        if self.debug {
            match self.raw_line.as_ref() {
                Some(r) => r.clone(),
                None => String::from("")
            }
        } else {
            unimplemented!();
        }
    }

    pub fn get_id(&self) -> Hash {
        compute_filter_id(
            self.csp.as_ref().map(String::as_str),
            self.mask,
            self.filter.as_ref().map(String::as_str),
            self.hostname.as_ref().map(String::as_str),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref(),
        )
    }

    // Lazily get the regex if the filter has one
    pub fn get_regex(&self) -> Result<Regex, FilterError> {
        if !self.is_regex() {
            return Err(FilterError::NoRegex);
        }
        // Create a new scope to contain the lifetime of the
        // dynamic borrow
        {
            let mut cache = self.regex.borrow_mut();
            if cache.is_some() {
                return Ok(cache.as_ref().unwrap().clone());
            }

            // TODO: eval impact - regex gets compiled in original code lazily
            let maybe_regex = if self.mask.contains(NetworkFilterMask::IS_REGEX) {
                let filter_regex = self.filter.as_ref().map(|f| {
                    compile_regex(f,
                        self.mask.contains(NetworkFilterMask::IS_RIGHT_ANCHOR),
                        self.mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR))
                });
                match filter_regex {
                    None => None,
                    Some(Ok(regex)) => Some(regex),
                    Some(Err(err)) => {
                        // if debug {
                        //     println!("Errored when parsing regex {}: {}", filter.as_ref().unwrap(), err);
                        // }
                        return Err(FilterError::RegexParsingError(err))
                    }
                }
            } else {
                None
            };

            *cache = maybe_regex;
        }
        // Recursive call to return the just-cached value.
        // Note that if we had not let the previous borrow
        // of the cache fall out of scope then the subsequent
        // recursive borrow would cause a dynamic thread panic.
        // This is the major hazard of using `RefCell`.
        self.get_regex()
    }

    pub fn get_fuzzy_signature(&mut self) -> &Vec<Hash> {
        if self.fuzzy_signature.is_none() {
            self.fuzzy_signature = match self.filter {
                Some(ref filter) if self.is_fuzzy() => Some(utils::create_fuzzy_signature(filter)),
                _ => Some(vec![]),
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
            let domains = self.opt_domains.as_ref().unwrap();
            let domain = domains.get(0).unwrap();
            tokens.push(*domain)
        }

        // Get tokens from filter
        self.filter.as_ref().map(|filter| {
            let skip_last_token = self.is_plain() && !self.is_right_anchor() && !self.is_fuzzy();
            let skip_first_token = self.is_right_anchor();
            let mut filter_tokens =
                utils::tokenize_filter(filter, skip_first_token, skip_last_token);

            tokens.append(&mut filter_tokens);
        });

        // Append tokens from hostname, if any
        self.hostname.as_ref().map(|hostname| {
            let mut hostname_tokens = utils::tokenize(&hostname);
            tokens.append(&mut hostname_tokens);
        });

        // If we got no tokens for the filter/hostname part, then we will dispatch
        // this filter in multiple buckets based on the domains option.
        if tokens.len() == 0 && self.opt_domains.is_some() && self.opt_not_domains.is_none() {
            self.opt_domains
                .as_ref()
                .unwrap_or(&vec![])
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

    pub fn is_cpt_allowed(&self, cpt: &request::RequestType) -> bool {
        match NetworkFilterMask::from(cpt) {
            NetworkFilterMask::UNMATCHED => self.from_any(),
            mask => self.mask.contains(mask),
        }
    }

    #[inline]
    fn get_cpt_mask(&self) -> NetworkFilterMask {
        self.mask & NetworkFilterMask::FROM_ANY
    }
    #[inline]
    fn is_fuzzy(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FUZZY_MATCH)
    }
    #[inline]
    pub fn is_exception(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_EXCEPTION)
    }
    #[inline]
    fn is_hostname_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR)
    }
    #[inline]
    fn is_right_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_RIGHT_ANCHOR)
    }
    #[inline]
    fn is_left_anchor(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR)
    }
    #[inline]
    fn match_case(&self) -> bool {
        self.mask.contains(NetworkFilterMask::MATCH_CASE)
    }
    #[inline]
    pub fn is_important(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_IMPORTANT)
    }
    #[inline]
    pub fn is_redirect(&self) -> bool {
        self.redirect.is_some()
    }
    #[inline]
    fn is_regex(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_REGEX)
    }
    #[inline]
    fn is_plain(&self) -> bool {
        !self.is_regex()
    }
    #[inline]
    pub fn is_csp(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_CSP)
    }
    #[inline]
    pub fn has_bug(&self) -> bool {
        self.bug.is_some()
    }
    #[inline]
    fn from_any(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_ANY)
    }
    #[inline]
    fn third_party(&self) -> bool {
        self.mask.contains(NetworkFilterMask::THIRD_PARTY)
    }
    #[inline]
    fn first_party(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FIRST_PARTY)
    }
    #[inline]
    fn from_image(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_IMAGE)
    }
    #[inline]
    fn from_media(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_MEDIA)
    }
    #[inline]
    fn from_object(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_OBJECT)
    }
    #[inline]
    fn from_other(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_OTHER)
    }
    #[inline]
    fn from_ping(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_PING)
    }
    #[inline]
    fn from_script(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_SCRIPT)
    }
    #[inline]
    fn from_stylesheet(&self) -> bool {
        self.get_cpt_mask()
            .contains(NetworkFilterMask::FROM_STYLESHEET)
    }
    #[inline]
    fn from_subdocument(&self) -> bool {
        self.get_cpt_mask()
            .contains(NetworkFilterMask::FROM_SUBDOCUMENT)
    }
    #[inline]
    fn from_websocket(&self) -> bool {
        self.get_cpt_mask()
            .contains(NetworkFilterMask::FROM_WEBSOCKET)
    }
    #[inline]
    fn from_http(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FROM_HTTP)
    }
    #[inline]
    fn from_https(&self) -> bool {
        self.mask.contains(NetworkFilterMask::FROM_HTTPS)
    }
    #[inline]
    fn from_xml_http_request(&self) -> bool {
        self.get_cpt_mask()
            .contains(NetworkFilterMask::FROM_XMLHTTPREQUEST)
    }
    #[inline]
    fn from_font(&self) -> bool {
        self.get_cpt_mask().contains(NetworkFilterMask::FROM_FONT)
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
fn compile_regex(filter_str: &str, is_right_anchor: bool, is_left_anchor: bool) -> Result<Regex, regex::Error> {
    lazy_static! {
      // Escape special regex characters: |.$+?{}()[]\
      static ref SPECIAL_RE: Regex = Regex::new(r"([\|\.\$\+\?\{\}\(\)\[\]])").unwrap();
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

    // FIXME: escape sequences are different, frequently errors on "unrecognised escape sequence", e.g.:
    // /^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/ ("\/" unrecognised)
    // /\:\/\/data\..*\\.com\/\[a-za-z0-9\]\{30,\}/ ("\:" unrecognised)
    let regex = Regex::new(&filter)?;
    Ok(regex)
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
    // Corner-case, if `filterHostname` is empty, then it's a match
    if filter_hostname.len() == 0 {
        return true;
    }

    // `filterHostname` cannot be longer than actual hostname
    if filter_hostname.len() > hostname.len() {
        return false;
    }

    // If they have the same len(), they should be equal
    if filter_hostname.len() == hostname.len() {
        return filter_hostname == hostname;
    }

    // Check if `filter_hostname` appears anywhere in `hostname`
    let hostname_found = hostname.find(filter_hostname);

    // No match
    if hostname_found.is_none() {
        return false;
    }

    let match_index = hostname_found.unwrap();

    // `filter_hostname` is a prefix of `hostname` and needs to match full a label.
    //
    // Examples (filter_hostname, hostname):
    //   * (foo, foo.com)
    //   * (sub.foo, sub.foo.com)
    if match_index == 0 {
        return filter_hostname.ends_with('.') || {
            let (_, remains) = hostname.split_at(filter_hostname.len());
            remains.starts_with('.')
        };
    }

    // `filter_hostname` is a suffix of `hostname`.
    //
    // Examples (filter_hostname, hostname):
    //    * (foo.com, sub.foo.com)
    //    * (com, foo.com)
    if hostname.len() == match_index + filter_hostname.len() {
        return filter_hostname.starts_with('.') || {
            let (_, remains) = hostname.split_at(match_index - 1);
            remains.starts_with('.')
        };
    }

    // `filter_hostname` is infix of `hostname` and needs match full labels
    return (filter_hostname.ends_with('.') || {
        let (_, remains) = hostname.split_at(filter_hostname.len());
        remains.starts_with('.')
    }) && (filter_hostname.starts_with('.') || {
        let (_, remains) = hostname.split_at(match_index - 1);
        remains.starts_with('.')
    });
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
    filter.fuzzy_signature.as_ref().map(|signature| {
        let request_signature = request.get_fuzzy_signature();

        if signature.len() > request_signature.len() {
            return false;
        }

        for filter_token in signature {
            // Find the occurrence of `c` in `request_signature`
            // Can assume fuzzy signatures are sorted
            if !request_signature.binary_search(filter_token).is_ok() {
                return false;
            }
        }

        true
    })
    .unwrap_or(true) // corner case of rulle having fuzzy option but no filter
    
}

// pattern
fn check_pattern_plain_filter_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    filter
        .filter
        .as_ref()
        .map(|f| request.url.find(f).is_some())
        .unwrap_or(true) // if no filter, we have a match
}

// pattern|
fn check_pattern_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    filter
        .filter
        .as_ref()
        .map(|f| request.url.ends_with(f))
        .unwrap_or(true)
}

// |pattern
fn check_pattern_left_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    filter
        .filter
        .as_ref()
        .map(|f| request.url.starts_with(f))
        .unwrap_or(true)
}

// |pattern|
fn check_pattern_left_right_anchor_filter(
    filter: &NetworkFilter,
    request: &request::Request,
) -> bool {
    filter
        .filter
        .as_ref()
        .map(|f| {
            &request.url == f
        })
        .unwrap_or(true)
}

// pattern*^
fn check_pattern_regex_filter_at(filter: &NetworkFilter, request: &request::Request, start_from: usize) -> bool {
    filter.get_regex().as_ref().map(|r| r.is_match(&request.url[start_from..]))
        .unwrap_or(true)
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
                check_pattern_regex_filter_at(filter, request, request.url.find(hostname).unwrap_or_default() + hostname.len())
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
                let matches = filter
                    .filter
                    .as_ref()
                    .map(|_| check_pattern_right_anchor_filter(filter, request))
                    // In this specific case it means that the specified hostname should match
                    // at the end of the hostname of the request. This allows to prevent false
                    // positive like ||foo.bar which would match https://foo.bar.baz where
                    // ||foo.bar^ would not.
                    .unwrap_or_else(|| {
                        request.hostname.len() == hostname.len()
                            || request.hostname.ends_with(hostname)
                    });
                matches
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
                filter
                    .filter
                    .as_ref()
                    .map(|f| {
                        request
                            .url
                            .splitn(2, hostname) // split at hostname - guaranteed to be there by above check
                            .nth(1) // take the part after hostname
                            // Since it must follow immediatly after the hostname and be a suffix of
                            // the URL, we conclude that filter must be equal to the part of the
                            // url following the hostname.
                            .map(|url_end| url_end == f)
                            .unwrap_or(false) // no match if nothing after hostname
                    })
                    .unwrap_or(true) // if no filter, we have a match
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
                filter
                    .filter
                    .as_ref()
                    .map(|f| {
                        request
                            .url
                            .splitn(2, hostname) // split at hostname - guaranteed to be there by above check
                            .nth(1) // take the part after hostname
                            // Since this is not a regex, the filter pattern must follow the hostname
                            // with nothing in between. So we extract the part of the URL following
                            // after hostname and will perform the matching on it.
                            .map(|url_end| utils::fast_starts_with(url_end, f))
                            .unwrap_or(false) // no match if nothing after hostname
                    })
                    .unwrap_or(true) // if no filter, we have a match
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
                filter
                    .filter
                    .as_ref()
                    .map(|f| {
                        request
                            .url
                            .splitn(2, hostname) // split at hostname - guaranteed to be there by above check
                            .nth(1) // take the part after hostname
                            .map(|url_end| url_end.find(f).is_some()) // We consider this a match if the plain patter (i.e.: filter) appears anywhere.
                            .unwrap_or(false) // no match if nothing after hostname
                    })
                    .unwrap_or(true) // if no filter, we have a match
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
            if is_anchored_by_hostname(hostname, &request.hostname) {
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
    } else if filter.is_regex() {
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

fn check_options(filter: &NetworkFilter, request: &request::Request) -> bool {
    // We first discard requests based on type, protocol and party. This is really
    // cheap and should be done first.
    if !filter.is_cpt_allowed(&request.request_type)
        || (request.is_https && !filter.from_https())
        || (request.is_http && !filter.from_http())
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
    if filter.opt_domains.is_some() {
        let domains = filter.opt_domains.as_ref().unwrap();
        if !utils::bin_lookup(&domains, request.source_hostname_hash)
            && !utils::bin_lookup(&domains, request.source_domain_hash)
        {
            return false;
        }
    }

    if filter.opt_not_domains.is_some() {
        let domains = filter.opt_not_domains.as_ref().unwrap();
        if utils::bin_lookup(&domains, request.source_hostname_hash)
            || utils::bin_lookup(&domains, request.source_domain_hash)
        {
            return false;
        }
    }

    return true;
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
        fn from(request: &NetworkFilter) -> NetworkFilterBreakdown {
            NetworkFilterBreakdown {
                filter: request.filter.as_ref().cloned(),
                bug: request.bug.as_ref().cloned(),
                csp: request.csp.as_ref().cloned(),
                hostname: request.hostname.as_ref().cloned(),
                opt_domains: request.opt_domains.as_ref().cloned(),
                opt_not_domains: request.opt_not_domains.as_ref().cloned(),
                redirect: request.redirect.as_ref().cloned(),

                // filter type
                is_fuzzy: request.is_fuzzy(),
                is_exception: request.is_exception(),
                is_hostname_anchor: request.is_hostname_anchor(),
                is_right_anchor: request.is_right_anchor(),
                is_left_anchor: request.is_left_anchor(),
                is_regex: request.is_regex(),
                is_csp: request.is_csp(),
                is_plain: request.is_plain(),
                is_important: request.is_important(),
                has_bug: request.has_bug(),

                // Options
                first_party: request.first_party(),
                from_any: request.from_any(),
                from_font: request.from_font(),
                from_image: request.from_image(),
                from_media: request.from_media(),
                from_object: request.from_object(),
                from_other: request.from_other(),
                from_ping: request.from_ping(),
                from_script: request.from_script(),
                from_stylesheet: request.from_stylesheet(),
                from_subdocument: request.from_subdocument(),
                from_websocket: request.from_websocket(),
                from_xml_http_request: request.from_xml_http_request(),
                match_case: request.match_case(),
                third_party: request.third_party(),
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
            assert_eq!(filter.from_image(), true);
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
            assert_eq!(
                filter.opt_domains,
                Some(domains)
            );
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
            let mut domains =vec![utils::fast_hash("foo"), utils::fast_hash("baz")];
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
            "badfilter",
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

}

#[cfg(test)]
mod match_tests {
    use super::*;

    #[test]
    fn is_anchored_by_hostname_works() {
        // matches empty hostname
        assert_eq!(is_anchored_by_hostname("", "foo.com"), true);

        // does not match when filter hostname is longer than hostname
        assert_eq!(is_anchored_by_hostname("bar.foo.com", "foo.com"), false);
        assert_eq!(is_anchored_by_hostname("b", ""), false);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.co"), false);

        // does not match if there is not match
        assert_eq!(is_anchored_by_hostname("bar", "foo.com"), false);

        // ## prefix match
        // matches exact match
        assert_eq!(is_anchored_by_hostname("", ""), true);
        assert_eq!(is_anchored_by_hostname("f", "f"), true);
        assert_eq!(is_anchored_by_hostname("foo", "foo"), true);
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com"), true);
        assert_eq!(is_anchored_by_hostname(".com", ".com"), true);
        assert_eq!(is_anchored_by_hostname("com.", "com."), true);

        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foo.com"), true);
        assert_eq!(is_anchored_by_hostname("foo.", "foo.com"), true);
        assert_eq!(is_anchored_by_hostname(".foo", ".foo.com"), true);
        assert_eq!(is_anchored_by_hostname(".foo.", ".foo.com"), true);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.com", "foo.com."), true);
        assert_eq!(is_anchored_by_hostname("foo.com.", "foo.com."), true);
        assert_eq!(is_anchored_by_hostname(".foo.com.", ".foo.com."), true);
        assert_eq!(is_anchored_by_hostname(".foo.com", ".foo.com"), true);

        assert_eq!(is_anchored_by_hostname("foo.bar", "foo.bar.com"), true);
        assert_eq!(is_anchored_by_hostname("foo.bar.", "foo.bar.com"), true);

        // does not match partial prefix
        // Single label
        assert_eq!(is_anchored_by_hostname("foo", "foobar.com"), false);
        assert_eq!(is_anchored_by_hostname("fo", "foo.com"), false);
        assert_eq!(is_anchored_by_hostname(".foo", "foobar.com"), false);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.bar", "foo.barbaz.com"), false);
        assert_eq!(
            is_anchored_by_hostname(".foo.bar", ".foo.barbaz.com"),
            false
        );

        // ## suffix match
        // matches partial
        // Single label
        assert_eq!(is_anchored_by_hostname("com", "foo.com"), true);
        assert_eq!(is_anchored_by_hostname(".com", "foo.com"), true);
        assert_eq!(is_anchored_by_hostname(".com.", "foo.com."), true);
        assert_eq!(is_anchored_by_hostname("com.", "foo.com."), true);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.com.", ".foo.com."), true);
        assert_eq!(is_anchored_by_hostname("foo.com", ".foo.com"), true);

        // does not match partial
        // Single label
        assert_eq!(is_anchored_by_hostname("om", "foo.com"), false);
        assert_eq!(is_anchored_by_hostname("com", "foocom"), false);

        // Multiple labels
        assert_eq!(is_anchored_by_hostname("foo.bar.com", "baz.bar.com"), false);
        assert_eq!(is_anchored_by_hostname("fo.bar.com", "foo.bar.com"), false);
        assert_eq!(is_anchored_by_hostname(".fo.bar.com", "foo.bar.com"), false);
        assert_eq!(is_anchored_by_hostname("bar.com", "foobar.com"), false);
        assert_eq!(is_anchored_by_hostname(".bar.com", "foobar.com"), false);

        // ## infix match
        // matches partial
        assert_eq!(is_anchored_by_hostname("bar", "foo.bar.com"), true);
        assert_eq!(is_anchored_by_hostname("bar.", "foo.bar.com"), true);
        assert_eq!(is_anchored_by_hostname(".bar.", "foo.bar.com"), true);
    }

    fn filter_match_url(filter: &str, url: &str, matching: bool) {
        let network_filter = NetworkFilter::parse(filter, true).unwrap();
        let request = request::Request::from_url(url).unwrap();
        
        assert!(network_filter.matches(&request) == matching,
            "Expected match={} for {} on {}", matching, filter, url);
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
        filter_match_url("|https://foo.com/|", "https://foo.com", true);
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
    fn check_unicode_handled() {        
        filter_match_url("||firstrowsports.li/frame/", "https://firstrowsports.li/frame/bar", true);
        filter_match_url("||fırstrowsports.eu/pu/", "https://fırstrowsports.eu/pu/foo", true);
        filter_match_url("||fırstrowsports.eu/pu/", "https://xn--frstrowsports-39b.eu/pu/foo", true);
        
        filter_match_url("||atđhe.net/pu/", "https://atđhe.net/pu/foo", true);
        filter_match_url("||atđhe.net/pu/", "https://xn--athe-1ua.net/pu/foo", true);
    }

    #[test]
    #[ignore]
    fn check_regex_escaping_handled() {
        // A rules that are not correctly escaped for rust Regex
        {
            // regex escaping "\/" unrecognised
            let filter = r#"/^https?:\/\/.*(bitly|bit)\.(com|ly)\/.*/$domain=123movies.com|1337x.to|1movies.is|activistpost.com|ancient-origins.net|couchtuner.onl|couchtuner.rocks|daclips.in|datpiff.com|dwatchseries.to|estream.nu|estream.to|estream.xyz|eztv.ag|eztv.io|eztv.tf|eztv.yt|fmovies.se|fmovies.taxi|fmovies.to|fmovies.today|fmovies.world|fullmatchesandshows.com|gomovies.sc|gorillavid.in|hdmoza.com|hdonline.is|healthline.com|kickass2.ch|kickass2.st|kimcartoon.to|limetorrents.info|megaup.net|monova.org|monova.to|movies.is|movies123.xyz|moviescouch.co|moviewatcher.is|newser.com|nowvideo.sx|nowwatchtvlive.ws|onhax.me|pirateiro.com|seedpeer.me|skidrowcrack.com|solarmovie.one|solarmoviez.ru|swatchseries.to|thegatewaypundit.com|thewatchseries.ac|tinypic.com|torlock.com|torrentdownload.ch|torrentdownloads.me|torrentfunk.com|torrentfunk2.com|torrentz2.eu|unblckd.org|unblocked.app|unblocked.lol|unblocked.si|watchcartoononline.io|watchseries.sk|watchsomuch.info|yesmovies.to|yify-movies.net|yify.bz|yifyddl.com|yifytorrent.co|ymovies.tv|yourbittorrent2.com|yts.am"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://bit.ly/bar";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "").unwrap();
            assert!(network_filter.matches(&request) == true,
            "Expected match for {} on {}", filter, url);
        }
        {
            // regex escaping "\:" unrecognised
            let filter = r#"/\:\/\/data.*\.com\/[a-zA-Z0-9]{30,}/$third-party,xmlhttprequest"#;
            let network_filter = NetworkFilter::parse(filter, true).unwrap();
            let url = "https://data.foo.com/9VjjrjU9Or2aqkb8PDiqTBnULPgeI48WmYEHkYer";
            let source = "http://123movies.com";
            let request = request::Request::from_urls(url, source, "xmlhttprequest").unwrap();
            assert!(network_filter.matches(&request) == true,
            "Expected match for {} on {}", filter, url);
        }
    }

}
