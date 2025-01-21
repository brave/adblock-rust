//! Filters that take effect at the network request level, including blocking and response
//! modification.

use memchr::{memchr as find_char, memmem, memrchr as find_char_reverse};
use once_cell::sync::Lazy;
use regex::{
    bytes::Regex as BytesRegex, bytes::RegexBuilder as BytesRegexBuilder,
    bytes::RegexSet as BytesRegexSet, bytes::RegexSetBuilder as BytesRegexSetBuilder, Regex,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::fmt;

use crate::lists::ParseOptions;
use crate::regex_manager::RegexManager;
use crate::request;
use crate::utils::{self, Hash};

pub(crate) const TOKENS_BUFFER_SIZE: usize = 200;

/// For now, only support `$removeparam` with simple alphanumeric/dash/underscore patterns.
static VALID_PARAM: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_\-]+$").unwrap());

#[derive(Debug, Error, PartialEq, Clone)]
pub enum NetworkFilterError {
    #[error("failed to parse filter")]
    FilterParseError,
    #[error("negated badfilter option")]
    NegatedBadFilter,
    #[error("negated important")]
    NegatedImportant,
    #[error("negated match-case")]
    NegatedOptionMatchCase,
    #[error("negated explicitcancel")]
    NegatedExplicitCancel,
    #[error("negated redirection")]
    NegatedRedirection,
    #[error("negated tag")]
    NegatedTag,
    #[error("negated generichide")]
    NegatedGenericHide,
    #[error("negated document")]
    NegatedDocument,
    #[error("generichide without exception")]
    GenericHideWithoutException,
    #[error("empty redirection")]
    EmptyRedirection,
    #[error("empty removeparam")]
    EmptyRemoveparam,
    #[error("negated removeparam")]
    NegatedRemoveparam,
    #[error("removeparam with exception")]
    RemoveparamWithException,
    #[error("removeparam regex unsupported")]
    RemoveparamRegexUnsupported,
    #[error("redirection url invalid")]
    RedirectionUrlInvalid,
    #[error("multiple modifier options")]
    MultipleModifierOptions,
    #[error("unrecognised option")]
    UnrecognisedOption,
    #[error("no regex")]
    NoRegex,
    #[error("full regex unsupported")]
    FullRegexUnsupported,
    #[error("regex parsing error")]
    RegexParsingError(regex::Error),
    #[error("punycode error")]
    PunycodeError,
    #[error("csp with content type")]
    CspWithContentType,
    #[error("match-case without full regex")]
    MatchCaseWithoutFullRegex,
    #[error("no supported domains")]
    NoSupportedDomains,
}

bitflags::bitflags! {
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
        const IS_REMOVEPARAM = 1 << 15;
        const THIRD_PARTY = 1 << 16;
        const FIRST_PARTY = 1 << 17;
        const IS_REDIRECT = 1 << 26;
        const BAD_FILTER = 1 << 27;
        const GENERIC_HIDE = 1 << 30;

        // Full document rules are not implied by negated types.
        const FROM_DOCUMENT = 1 << 29;

        // Kind of pattern
        const IS_REGEX = 1 << 18;
        const IS_LEFT_ANCHOR = 1 << 19;
        const IS_RIGHT_ANCHOR = 1 << 20;
        const IS_HOSTNAME_ANCHOR = 1 << 21;
        const IS_EXCEPTION = 1 << 22;
        const IS_CSP = 1 << 23;
        const IS_COMPLETE_REGEX = 1 << 24;
        const IS_HOSTNAME_REGEX = 1 << 28;

        // Specifies that a redirect rule should also create a corresponding block rule.
        // This is used to avoid returning two separate rules from `NetworkFilter::parse`.
        const ALSO_BLOCK_REDIRECT = 1 << 31;

        // "Other" network request types
        const UNMATCHED = 1 << 25;

        // Includes all request types that are implied by any negated types.
        const FROM_NETWORK_TYPES = Self::FROM_FONT.bits |
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

        // Includes all remaining types, not implied by any negated types.
        // TODO Could also include popup, inline-font, inline-script
        const FROM_ALL_TYPES = Self::FROM_NETWORK_TYPES.bits |
            Self::FROM_DOCUMENT.bits;

        // Unless filter specifies otherwise, all these options are set by default
        const DEFAULT_OPTIONS = Self::FROM_NETWORK_TYPES.bits |
            Self::FROM_HTTP.bits |
            Self::FROM_HTTPS.bits |
            Self::THIRD_PARTY.bits |
            Self::FIRST_PARTY.bits;

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
            request::RequestType::Beacon => NetworkFilterMask::FROM_PING,
            request::RequestType::Csp => NetworkFilterMask::UNMATCHED,
            request::RequestType::Document => NetworkFilterMask::FROM_DOCUMENT,
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

#[derive(Debug, Clone)]
pub enum CompiledRegex {
    Compiled(BytesRegex),
    CompiledSet(BytesRegexSet),
    MatchAll,
    RegexParsingError(regex::Error),
}

impl CompiledRegex {
    pub fn is_match(&self, pattern: &str) -> bool {
        match &self {
            CompiledRegex::MatchAll => true, // simple case for matching everything, e.g. for empty filter
            CompiledRegex::RegexParsingError(_e) => false, // no match if regex didn't even compile
            CompiledRegex::Compiled(r) => r.is_match(pattern.as_bytes()),
            CompiledRegex::CompiledSet(r) => {
                // let matches: Vec<_> = r.matches(pattern).into_iter().collect();
                // println!("Matching {} against RegexSet: {:?}", pattern, matches);
                r.is_match(pattern.as_bytes())
            }
        }
    }
}

impl fmt::Display for CompiledRegex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            CompiledRegex::MatchAll => write!(f, ".*"), // simple case for matching everything, e.g. for empty filter
            CompiledRegex::RegexParsingError(_e) => write!(f, "ERROR"), // no match if regex didn't even compile
            CompiledRegex::Compiled(r) => write!(f, "{}", r.as_str()),
            CompiledRegex::CompiledSet(r) => write!(f, "{}", r.patterns().join(" | ")),
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

#[derive(Clone, Copy)]
enum NetworkFilterLeftAnchor {
    /// A `||` token, which represents a match to the start of a domain or subdomain segment.
    DoublePipe,
    /// A `|` token, which represents a match to the exact start of the URL.
    SinglePipe,
}

#[derive(Clone, Copy)]
enum NetworkFilterRightAnchor {
    /// A `|` token, which represents a match to the exact end of the URL.
    SinglePipe,
}

/// Pattern for a network filter, describing what URLs to match against.
#[derive(Clone)]
struct NetworkFilterPattern {
    left_anchor: Option<NetworkFilterLeftAnchor>,
    pattern: String,
    right_anchor: Option<NetworkFilterRightAnchor>,
}

/// Any option that appears on the right side of a network filter as initiated by a `$` character.
/// All `bool` arguments below are `true` if the option stands alone, or `false` if the option is
/// negated using a prepended `~`.
#[derive(Clone)]
enum NetworkFilterOption {
    Domain(Vec<(bool, String)>),
    Badfilter,
    Important,
    MatchCase,
    ThirdParty(bool),
    FirstParty(bool),
    Tag(String),
    Redirect(String),
    RedirectRule(String),
    Csp(Option<String>),
    Removeparam(String),
    Generichide,
    Document,
    Image(bool),
    Media(bool),
    Object(bool),
    Other(bool),
    Ping(bool),
    Script(bool),
    Stylesheet(bool),
    Subdocument(bool),
    XmlHttpRequest(bool),
    Websocket(bool),
    Font(bool),
}

impl NetworkFilterOption {
    pub fn is_content_type(&self) -> bool {
        matches!(
            self,
            Self::Document
                | Self::Image(..)
                | Self::Media(..)
                | Self::Object(..)
                | Self::Other(..)
                | Self::Ping(..)
                | Self::Script(..)
                | Self::Stylesheet(..)
                | Self::Subdocument(..)
                | Self::XmlHttpRequest(..)
                | Self::Websocket(..)
                | Self::Font(..)
        )
    }

    pub fn is_redirection(&self) -> bool {
        matches!(self, Self::Redirect(..) | Self::RedirectRule(..))
    }
}

/// Abstract syntax representation of a network filter. This representation can fully specify the
/// string representation of a filter as written, with the exception of aliased options like `1p`
/// or `ghide`. This allows separation of concerns between parsing and interpretation.
struct AbstractNetworkFilter {
    exception: bool,
    pattern: NetworkFilterPattern,
    options: Option<Vec<NetworkFilterOption>>,
}

impl AbstractNetworkFilter {
    fn parse(line: &str) -> Result<Self, NetworkFilterError> {
        let mut filter_index_start: usize = 0;
        let mut filter_index_end: usize = line.len();

        let mut exception = false;
        if line.starts_with("@@") {
            filter_index_start += 2;
            exception = true;
        }

        let maybe_options_index: Option<usize> = find_char_reverse(b'$', line.as_bytes());

        let mut options = None;
        if let Some(options_index) = maybe_options_index {
            filter_index_end = options_index;

            // slicing here is safe; the first byte after '$' will be a character boundary
            let raw_options = &line[filter_index_end + 1..];

            options = Some(parse_filter_options(raw_options)?);
        }

        let left_anchor = if line[filter_index_start..].starts_with("||") {
            filter_index_start += 2;
            Some(NetworkFilterLeftAnchor::DoublePipe)
        } else if line[filter_index_start..].starts_with('|') {
            filter_index_start += 1;
            Some(NetworkFilterLeftAnchor::SinglePipe)
        } else {
            None
        };

        let right_anchor = if filter_index_end > 0
            && filter_index_end > filter_index_start
            && line[..filter_index_end].ends_with('|')
        {
            filter_index_end -= 1;
            Some(NetworkFilterRightAnchor::SinglePipe)
        } else {
            None
        };

        let pattern = &line[filter_index_start..filter_index_end];

        Ok(AbstractNetworkFilter {
            exception,
            pattern: NetworkFilterPattern {
                left_anchor,
                pattern: pattern.to_string(),
                right_anchor,
            },
            options,
        })
    }
}

fn parse_filter_options(raw_options: &str) -> Result<Vec<NetworkFilterOption>, NetworkFilterError> {
    let mut result = vec![];

    for raw_option in raw_options.split(',') {
        // Check for negation: ~option
        let negation = raw_option.starts_with('~');
        let maybe_negated_option = raw_option.trim_start_matches('~');

        // Check for options: option=value1|value2
        let mut option_and_values = maybe_negated_option.splitn(2, '=');
        let (option, value) = (
            option_and_values.next().unwrap(),
            option_and_values.next().unwrap_or_default(),
        );

        result.push(match (option, negation) {
            ("domain", _) | ("from", _) => {
                let domains: Vec<(bool, String)> = value
                    .split('|')
                    .map(|domain| {
                        if let Some(negated_domain) = domain.strip_prefix('~') {
                            (false, negated_domain.to_string())
                        } else {
                            (true, domain.to_string())
                        }
                    })
                    .filter(|(_, d)| !(d.starts_with('/') && d.ends_with('/')))
                    .collect();
                if domains.is_empty() {
                    return Err(NetworkFilterError::NoSupportedDomains);
                }
                NetworkFilterOption::Domain(domains)
            }
            ("badfilter", true) => return Err(NetworkFilterError::NegatedBadFilter),
            ("badfilter", false) => NetworkFilterOption::Badfilter,
            ("important", true) => return Err(NetworkFilterError::NegatedImportant),
            ("important", false) => NetworkFilterOption::Important,
            ("match-case", true) => return Err(NetworkFilterError::NegatedOptionMatchCase),
            ("match-case", false) => NetworkFilterOption::MatchCase,
            ("third-party", negated) | ("3p", negated) => NetworkFilterOption::ThirdParty(!negated),
            ("first-party", negated) | ("1p", negated) => NetworkFilterOption::FirstParty(!negated),
            ("tag", true) => return Err(NetworkFilterError::NegatedTag),
            ("tag", false) => NetworkFilterOption::Tag(String::from(value)),
            ("redirect", true) => return Err(NetworkFilterError::NegatedRedirection),
            ("redirect", false) => {
                // Ignore this filter if no redirection resource is specified
                if value.is_empty() {
                    return Err(NetworkFilterError::EmptyRedirection);
                }

                NetworkFilterOption::Redirect(String::from(value))
            }
            ("redirect-rule", true) => return Err(NetworkFilterError::NegatedRedirection),
            ("redirect-rule", false) => {
                if value.is_empty() {
                    return Err(NetworkFilterError::EmptyRedirection);
                }

                NetworkFilterOption::RedirectRule(String::from(value))
            }
            ("csp", _) => NetworkFilterOption::Csp(if !value.is_empty() {
                Some(String::from(value))
            } else {
                None
            }),
            ("removeparam", true) => return Err(NetworkFilterError::NegatedRemoveparam),
            ("removeparam", false) => {
                if value.is_empty() {
                    return Err(NetworkFilterError::EmptyRemoveparam);
                }
                if !VALID_PARAM.is_match(value) {
                    return Err(NetworkFilterError::RemoveparamRegexUnsupported);
                }
                NetworkFilterOption::Removeparam(String::from(value))
            }
            ("generichide", true) | ("ghide", true) => {
                return Err(NetworkFilterError::NegatedGenericHide)
            }
            ("generichide", false) | ("ghide", false) => NetworkFilterOption::Generichide,
            ("document", true) | ("doc", true) => return Err(NetworkFilterError::NegatedDocument),
            ("document", false) | ("doc", false) => NetworkFilterOption::Document,
            ("image", negated) => NetworkFilterOption::Image(!negated),
            ("media", negated) => NetworkFilterOption::Media(!negated),
            ("object", negated) | ("object-subrequest", negated) => {
                NetworkFilterOption::Object(!negated)
            }
            ("other", negated) => NetworkFilterOption::Other(!negated),
            ("ping", negated) | ("beacon", negated) => NetworkFilterOption::Ping(!negated),
            ("script", negated) => NetworkFilterOption::Script(!negated),
            ("stylesheet", negated) | ("css", negated) => NetworkFilterOption::Stylesheet(!negated),
            ("subdocument", negated) | ("frame", negated) => {
                NetworkFilterOption::Subdocument(!negated)
            }
            ("xmlhttprequest", negated) | ("xhr", negated) => {
                NetworkFilterOption::XmlHttpRequest(!negated)
            }
            ("websocket", negated) => NetworkFilterOption::Websocket(!negated),
            ("font", negated) => NetworkFilterOption::Font(!negated),
            (_, _) => return Err(NetworkFilterError::UnrecognisedOption),
        });
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFilter {
    pub mask: NetworkFilterMask,
    pub filter: FilterPart,
    pub opt_domains: Option<Vec<Hash>>,
    pub opt_not_domains: Option<Vec<Hash>>,
    /// Used for `$redirect`, `$redirect-rule`, `$csp`, and `$removeparam` - only one of which is
    /// supported per-rule.
    pub modifier_option: Option<String>,
    pub hostname: Option<String>,
    pub(crate) tag: Option<String>,

    pub raw_line: Option<Box<String>>,

    pub id: Hash,

    // All domain option values (their hashes) OR'ed together to quickly dismiss mis-matches
    pub opt_domains_union: Option<Hash>,
    pub opt_not_domains_union: Option<Hash>,
}

// TODO - restrict the API so that this is always true - i.e. lazy-calculate IDs from actual data,
// prevent field access, and don't load the ID from the serialized format.
/// The ID of a filter is assumed to be correctly calculated for the purposes of this
/// implementation.
impl PartialEq for NetworkFilter {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Filters are sorted by ID to preserve a stable ordering of data in the serialized format.
impl PartialOrd for NetworkFilter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

/// Ensure that no invalid option combinations were provided for a filter.
fn validate_options(options: &[NetworkFilterOption]) -> Result<(), NetworkFilterError> {
    let mut has_csp = false;
    let mut has_content_type = false;
    let mut modifier_options = 0;
    for option in options {
        if matches!(option, NetworkFilterOption::Csp(..)) {
            has_csp = true;
            modifier_options += 1;
        } else if option.is_content_type() {
            has_content_type = true;
        } else if option.is_redirection() || matches!(option, NetworkFilterOption::Removeparam(..))
        {
            modifier_options += 1;
        }
    }
    if has_csp && has_content_type {
        return Err(NetworkFilterError::CspWithContentType);
    }
    if modifier_options > 1 {
        return Err(NetworkFilterError::MultipleModifierOptions);
    }

    Ok(())
}

impl NetworkFilter {
    pub fn parse(line: &str, debug: bool, _opts: ParseOptions) -> Result<Self, NetworkFilterError> {
        let parsed = AbstractNetworkFilter::parse(line)?;

        // Represent options as a bitmask
        let mut mask: NetworkFilterMask = NetworkFilterMask::THIRD_PARTY
            | NetworkFilterMask::FIRST_PARTY
            | NetworkFilterMask::FROM_HTTPS
            | NetworkFilterMask::FROM_HTTP;

        // Temporary masks for positive (e.g.: $script) and negative (e.g.: $~script)
        // content type options.
        let mut cpt_mask_positive: NetworkFilterMask = NetworkFilterMask::NONE;
        let mut cpt_mask_negative: NetworkFilterMask = NetworkFilterMask::NONE;

        let mut hostname: Option<String> = None;

        let mut opt_domains: Option<Vec<Hash>> = None;
        let mut opt_not_domains: Option<Vec<Hash>> = None;
        let mut opt_domains_union: Option<Hash> = None;
        let mut opt_not_domains_union: Option<Hash> = None;

        let mut modifier_option: Option<String> = None;
        let mut tag: Option<String> = None;

        if parsed.exception {
            mask.set(NetworkFilterMask::IS_EXCEPTION, true);
        }

        if let Some(options) = parsed.options {
            validate_options(&options)?;

            macro_rules! apply_content_type {
                ($content_type:ident, $enabled:ident) => {
                    if $enabled {
                        cpt_mask_positive.set(NetworkFilterMask::$content_type, true);
                    } else {
                        cpt_mask_negative.set(NetworkFilterMask::$content_type, true);
                    }
                };
            }

            options.into_iter().for_each(|option| {
                match option {
                    NetworkFilterOption::Domain(mut domains) => {
                        // Some rules have duplicate domain options - avoid including duplicates
                        // Benchmarking doesn't indicate signficant performance degradation across the entire easylist
                        domains.sort_unstable();
                        domains.dedup();
                        let mut opt_domains_array: Vec<Hash> = vec![];
                        let mut opt_not_domains_array: Vec<Hash> = vec![];

                        for (enabled, domain) in domains {
                            let domain_hash = utils::fast_hash(&domain);
                            if !enabled {
                                opt_not_domains_array.push(domain_hash);
                            } else {
                                opt_domains_array.push(domain_hash);
                            }
                        }

                        if !opt_domains_array.is_empty() {
                            opt_domains_array.sort_unstable();
                            opt_domains_union =
                                Some(opt_domains_array.iter().fold(0, |acc, x| acc | x));
                            opt_domains = Some(opt_domains_array);
                        }
                        if !opt_not_domains_array.is_empty() {
                            opt_not_domains_array.sort_unstable();
                            opt_not_domains_union =
                                Some(opt_not_domains_array.iter().fold(0, |acc, x| acc | x));
                            opt_not_domains = Some(opt_not_domains_array);
                        }
                    }
                    NetworkFilterOption::Badfilter => mask.set(NetworkFilterMask::BAD_FILTER, true),
                    NetworkFilterOption::Important => {
                        mask.set(NetworkFilterMask::IS_IMPORTANT, true)
                    }
                    NetworkFilterOption::MatchCase => mask.set(NetworkFilterMask::MATCH_CASE, true),
                    NetworkFilterOption::ThirdParty(false)
                    | NetworkFilterOption::FirstParty(true) => {
                        mask.set(NetworkFilterMask::THIRD_PARTY, false)
                    }
                    NetworkFilterOption::ThirdParty(true)
                    | NetworkFilterOption::FirstParty(false) => {
                        mask.set(NetworkFilterMask::FIRST_PARTY, false)
                    }
                    NetworkFilterOption::Tag(value) => tag = Some(value),
                    NetworkFilterOption::Redirect(value) => {
                        mask.set(NetworkFilterMask::IS_REDIRECT, true);
                        mask.set(NetworkFilterMask::ALSO_BLOCK_REDIRECT, true);
                        modifier_option = Some(value);
                    }
                    NetworkFilterOption::RedirectRule(value) => {
                        mask.set(NetworkFilterMask::IS_REDIRECT, true);
                        modifier_option = Some(value);
                    }
                    NetworkFilterOption::Removeparam(value) => {
                        mask.set(NetworkFilterMask::IS_REMOVEPARAM, true);
                        modifier_option = Some(value);
                    }
                    NetworkFilterOption::Csp(value) => {
                        mask.set(NetworkFilterMask::IS_CSP, true);
                        // CSP rules can never have content types, and should always match against
                        // subdocument and document rules. Rules do not match against document
                        // requests by default, so this must be explictly added.
                        mask.set(NetworkFilterMask::FROM_DOCUMENT, true);
                        modifier_option = value;
                    }
                    NetworkFilterOption::Generichide => {
                        mask.set(NetworkFilterMask::GENERIC_HIDE, true)
                    }
                    NetworkFilterOption::Document => {
                        cpt_mask_positive.set(NetworkFilterMask::FROM_DOCUMENT, true)
                    }
                    NetworkFilterOption::Image(enabled) => apply_content_type!(FROM_IMAGE, enabled),
                    NetworkFilterOption::Media(enabled) => apply_content_type!(FROM_MEDIA, enabled),
                    NetworkFilterOption::Object(enabled) => {
                        apply_content_type!(FROM_OBJECT, enabled)
                    }
                    NetworkFilterOption::Other(enabled) => apply_content_type!(FROM_OTHER, enabled),
                    NetworkFilterOption::Ping(enabled) => apply_content_type!(FROM_PING, enabled),
                    NetworkFilterOption::Script(enabled) => {
                        apply_content_type!(FROM_SCRIPT, enabled)
                    }
                    NetworkFilterOption::Stylesheet(enabled) => {
                        apply_content_type!(FROM_STYLESHEET, enabled)
                    }
                    NetworkFilterOption::Subdocument(enabled) => {
                        apply_content_type!(FROM_SUBDOCUMENT, enabled)
                    }
                    NetworkFilterOption::XmlHttpRequest(enabled) => {
                        apply_content_type!(FROM_XMLHTTPREQUEST, enabled)
                    }
                    NetworkFilterOption::Websocket(enabled) => {
                        apply_content_type!(FROM_WEBSOCKET, enabled)
                    }
                    NetworkFilterOption::Font(enabled) => apply_content_type!(FROM_FONT, enabled),
                }
            });
        }

        mask |= cpt_mask_positive;

        // If any negated "network" types were set, then implicitly enable all network types.
        // The negated types will be applied later.
        //
        // This doesn't apply to removeparam filters.
        if !mask.contains(NetworkFilterMask::IS_REMOVEPARAM)
            && (cpt_mask_negative & NetworkFilterMask::FROM_NETWORK_TYPES)
                != NetworkFilterMask::NONE
        {
            mask |= NetworkFilterMask::FROM_NETWORK_TYPES;
        }
        // If no positive types were set, then the filter should apply to all network types.
        if (cpt_mask_positive & NetworkFilterMask::FROM_ALL_TYPES).is_empty() {
            // Removeparam is again a special case.
            if mask.contains(NetworkFilterMask::IS_REMOVEPARAM) {
                mask |= NetworkFilterMask::FROM_DOCUMENT
                    | NetworkFilterMask::FROM_SUBDOCUMENT
                    | NetworkFilterMask::FROM_XMLHTTPREQUEST;
            } else {
                mask |= NetworkFilterMask::FROM_NETWORK_TYPES;
            }
        }

        match parsed.pattern.left_anchor {
            Some(NetworkFilterLeftAnchor::DoublePipe) => {
                mask.set(NetworkFilterMask::IS_HOSTNAME_ANCHOR, true)
            }
            Some(NetworkFilterLeftAnchor::SinglePipe) => {
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true)
            }
            None => (),
        }

        // TODO these need to actually be handled differently than trailing `^`.
        let mut end_url_anchor = false;
        if let Some(NetworkFilterRightAnchor::SinglePipe) = parsed.pattern.right_anchor {
            mask.set(NetworkFilterMask::IS_RIGHT_ANCHOR, true);
            end_url_anchor = true;
        }

        let pattern = &parsed.pattern.pattern;

        let is_regex = check_is_regex(pattern);
        mask.set(NetworkFilterMask::IS_REGEX, is_regex);

        if pattern.starts_with('/') && pattern.ends_with('/') && pattern.len() > 1 {
            #[cfg(feature = "full-regex-handling")]
            {
                mask.set(NetworkFilterMask::IS_COMPLETE_REGEX, true);
            }

            #[cfg(not(feature = "full-regex-handling"))]
            {
                return Err(NetworkFilterError::FullRegexUnsupported);
            }
        } else {
            if !(mask & NetworkFilterMask::MATCH_CASE).is_empty() {
                return Err(NetworkFilterError::MatchCaseWithoutFullRegex);
            }
        }

        let (mut filter_index_start, mut filter_index_end) = (0, pattern.len());

        if let Some(NetworkFilterLeftAnchor::DoublePipe) = parsed.pattern.left_anchor {
            if is_regex {
                // Split at the first '/', '*' or '^' character to get the hostname
                // and then the pattern.
                // TODO - this could be made more efficient if we could match between two
                // indices. Once again, we have to do more work than is really needed.
                static SEPARATOR: Lazy<Regex> = Lazy::new(|| Regex::new("[/^*]").unwrap());
                if let Some(first_separator) = SEPARATOR.find(pattern) {
                    let first_separator_start = first_separator.start();
                    // NOTE: `first_separator` shall never be -1 here since `IS_REGEX` is true.
                    // This means there must be at least an occurrence of `*` or `^`
                    // somewhere.

                    // If the first separator is a wildcard, included in in hostname
                    if first_separator_start < pattern.len()
                        && pattern[first_separator_start..=first_separator_start].starts_with('*')
                    {
                        mask.set(NetworkFilterMask::IS_HOSTNAME_REGEX, true);
                    }

                    hostname = Some(String::from(&pattern[..first_separator_start]));
                    filter_index_start = first_separator_start;

                    // If the only symbol remaining for the selector is '^' then ignore it
                    // but set the filter as right anchored since there should not be any
                    // other label on the right
                    if filter_index_end - filter_index_start == 1
                        && pattern[filter_index_start..].starts_with('^')
                    {
                        mask.set(NetworkFilterMask::IS_REGEX, false);
                        filter_index_start = filter_index_end;
                        mask.set(NetworkFilterMask::IS_RIGHT_ANCHOR, true);
                    } else {
                        mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
                        mask.set(
                            NetworkFilterMask::IS_REGEX,
                            check_is_regex(&pattern[filter_index_start..filter_index_end]),
                        );
                    }
                }
            } else {
                // Look for next /
                let slash_index = find_char(b'/', pattern.as_bytes());
                slash_index
                    .map(|i| {
                        hostname = Some(String::from(&pattern[..i]));
                        filter_index_start += i;
                        mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, true);
                    })
                    .or_else(|| {
                        hostname = Some(String::from(pattern));
                        filter_index_start = filter_index_end;
                        None
                    });
            }
        }

        // Remove trailing '*'
        if filter_index_end > filter_index_start && pattern.ends_with('*') {
            filter_index_end -= 1;
        }

        // Remove leading '*' if the filter is not hostname anchored.
        if filter_index_end > filter_index_start && pattern[filter_index_start..].starts_with('*') {
            mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
            filter_index_start += 1;
        }

        // Transform filters on protocol (http, https, ws)
        if mask.contains(NetworkFilterMask::IS_LEFT_ANCHOR) {
            if filter_index_end == filter_index_start + 5
                && pattern[filter_index_start..].starts_with("ws://")
            {
                mask.set(NetworkFilterMask::FROM_WEBSOCKET, true);
                mask.set(NetworkFilterMask::FROM_HTTP, false);
                mask.set(NetworkFilterMask::FROM_HTTPS, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end == filter_index_start + 7
                && pattern[filter_index_start..].starts_with("http://")
            {
                mask.set(NetworkFilterMask::FROM_HTTP, true);
                mask.set(NetworkFilterMask::FROM_HTTPS, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end == filter_index_start + 8
                && pattern[filter_index_start..].starts_with("https://")
            {
                mask.set(NetworkFilterMask::FROM_HTTPS, true);
                mask.set(NetworkFilterMask::FROM_HTTP, false);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            } else if filter_index_end == filter_index_start + 8
                && pattern[filter_index_start..].starts_with("http*://")
            {
                mask.set(NetworkFilterMask::FROM_HTTPS, true);
                mask.set(NetworkFilterMask::FROM_HTTP, true);
                mask.set(NetworkFilterMask::IS_LEFT_ANCHOR, false);
                filter_index_start = filter_index_end;
            }
        }

        let filter: Option<String> = if filter_index_end > filter_index_start {
            let filter_str = &pattern[filter_index_start..filter_index_end];
            mask.set(NetworkFilterMask::IS_REGEX, check_is_regex(filter_str));
            if mask.contains(NetworkFilterMask::MATCH_CASE) {
                Some(String::from(filter_str))
            } else {
                Some(filter_str.to_ascii_lowercase())
            }
        } else {
            None
        };

        // TODO: ignore hostname anchor is not hostname provided

        let hostname_decoded = hostname
            .map(|host| {
                let hostname_normalised = if mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR) {
                    host.trim_start_matches("www.")
                } else {
                    &host
                };

                let lowercase = hostname_normalised.to_lowercase();
                let hostname = if lowercase.is_ascii() {
                    lowercase
                } else {
                    idna::domain_to_ascii(&lowercase)
                        .map_err(|_| NetworkFilterError::PunycodeError)?
                };
                Ok(hostname)
            })
            .transpose();

        if mask.contains(NetworkFilterMask::GENERIC_HIDE) && !parsed.exception {
            return Err(NetworkFilterError::GenericHideWithoutException);
        }

        if mask.contains(NetworkFilterMask::IS_REMOVEPARAM) && parsed.exception {
            return Err(NetworkFilterError::RemoveparamWithException);
        }

        // uBlock Origin would block main document `https://example.com` requests with all of the
        // following filters:
        // - ||example.com
        // - ||example.com/
        // - example.com
        // - https://example.com
        // However, it relies on checking the URL post-match against information from the matched
        // filter, which isn't saved in Brave unless running with filter lists compiled in "debug"
        // mode. Instead, we apply the implicit document matching more strictly, only for hostname
        // filters of the form `||example.com^`.
        if (cpt_mask_positive & NetworkFilterMask::FROM_ALL_TYPES).is_empty()
            && (cpt_mask_negative & NetworkFilterMask::FROM_ALL_TYPES).is_empty()
            && mask.contains(NetworkFilterMask::IS_HOSTNAME_ANCHOR)
            && mask.contains(NetworkFilterMask::IS_RIGHT_ANCHOR)
            && !end_url_anchor
            && !mask.contains(NetworkFilterMask::IS_REMOVEPARAM)
        {
            mask |= NetworkFilterMask::FROM_ALL_TYPES;
        }
        // Finally, apply any explicitly negated request types
        mask &= !cpt_mask_negative;

        Ok(NetworkFilter {
            filter: if let Some(simple_filter) = filter {
                FilterPart::Simple(simple_filter)
            } else {
                FilterPart::Empty
            },
            hostname: hostname_decoded?,
            mask,
            opt_domains,
            opt_not_domains,
            tag,
            raw_line: if debug {
                Some(Box::new(String::from(line)))
            } else {
                None
            },
            modifier_option,
            id: utils::fast_hash(line),
            opt_domains_union,
            opt_not_domains_union,
        })
    }

    /// Given a hostname, produces an equivalent filter parsed from the form `"||hostname^"`, to
    /// emulate the behavior of hosts-style blocking.
    pub fn parse_hosts_style(hostname: &str, debug: bool) -> Result<Self, NetworkFilterError> {
        // Make sure the hostname doesn't contain any invalid characters
        static INVALID_CHARS: Lazy<Regex> =
            Lazy::new(|| Regex::new("[/^*!?$&(){}\\[\\]+=~`\\s|@,'\"><:;]").unwrap());
        if INVALID_CHARS.is_match(hostname) {
            return Err(NetworkFilterError::FilterParseError);
        }

        // This shouldn't be used to block an entire TLD, and the hostname shouldn't end with a dot
        if find_char(b'.', hostname.as_bytes()).is_none()
            || (hostname.starts_with('.') && find_char(b'.', hostname[1..].as_bytes()).is_none())
            || hostname.ends_with('.')
        {
            return Err(NetworkFilterError::FilterParseError);
        }

        // Normalize the hostname to punycode and parse it as a `||hostname^` rule.
        let normalized_host = hostname.to_lowercase();
        let normalized_host = normalized_host.trim_start_matches("www.");

        let mut hostname = "||".to_string();
        if normalized_host.is_ascii() {
            hostname.push_str(normalized_host);
        } else {
            hostname.push_str(
                &idna::domain_to_ascii(normalized_host)
                    .map_err(|_| NetworkFilterError::PunycodeError)?,
            );
        }
        hostname.push('^');

        NetworkFilter::parse(&hostname, debug, Default::default())
    }

    pub fn get_id_without_badfilter(&self) -> Hash {
        let mut mask = self.mask;
        mask.set(NetworkFilterMask::BAD_FILTER, false);
        compute_filter_id(
            self.modifier_option.as_deref(),
            mask,
            self.filter.string_view().as_deref(),
            self.hostname.as_deref(),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref(),
        )
    }

    pub fn get_id(&self) -> Hash {
        compute_filter_id(
            self.modifier_option.as_deref(),
            self.mask,
            self.filter.string_view().as_deref(),
            self.hostname.as_deref(),
            self.opt_domains.as_ref(),
            self.opt_not_domains.as_ref(),
        )
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
                        (self.is_plain() || self.is_regex()) && !self.is_right_anchor();
                    let skip_first_token = self.is_right_anchor();

                    let mut filter_tokens =
                        utils::tokenize_filter(f, skip_first_token, skip_last_token);

                    tokens.append(&mut filter_tokens);
                }
            }
            FilterPart::AnyOf(_) => (), // across AnyOf set of filters no single token is guaranteed to match to a request
            _ => (),
        }

        // Append tokens from hostname, if any
        if !self.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX) {
            if let Some(hostname) = self.hostname.as_ref() {
                let mut hostname_tokens = utils::tokenize(hostname);
                tokens.append(&mut hostname_tokens);
            }
        }

        if tokens.is_empty() && self.mask.contains(NetworkFilterMask::IS_REMOVEPARAM) {
            if let Some(removeparam) = &self.modifier_option {
                if VALID_PARAM.is_match(removeparam) {
                    let mut param_tokens = utils::tokenize(&removeparam.to_ascii_lowercase());
                    tokens.append(&mut param_tokens);
                }
            }
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
        self.mask.contains(NetworkFilterMask::IS_REDIRECT)
    }

    pub fn is_removeparam(&self) -> bool {
        self.mask.contains(NetworkFilterMask::IS_REMOVEPARAM)
    }

    pub fn also_block_redirect(&self) -> bool {
        self.mask.contains(NetworkFilterMask::ALSO_BLOCK_REDIRECT)
    }

    pub fn is_badfilter(&self) -> bool {
        self.mask.contains(NetworkFilterMask::BAD_FILTER)
    }

    pub fn is_generic_hide(&self) -> bool {
        self.mask.contains(NetworkFilterMask::GENERIC_HIDE)
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

    fn check_cpt_allowed(&self, cpt: &request::RequestType) -> bool {
        match NetworkFilterMask::from(cpt) {
            // TODO this is not ideal, but required to allow regexed exception rules without an
            // explicit `$document` option to apply uBO-style.
            // See also: https://github.com/uBlockOrigin/uBlock-issues/issues/1501
            NetworkFilterMask::FROM_DOCUMENT => {
                self.mask.contains(NetworkFilterMask::FROM_DOCUMENT) || self.is_exception()
            }
            mask => self.mask.contains(mask),
        }
    }
}

impl fmt::Display for NetworkFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.raw_line.as_ref() {
            Some(r) => write!(f, "{}", r.clone()),
            None => write!(f, "NetworkFilter"),
        }
    }
}

pub trait NetworkMatchable {
    fn matches(&self, request: &request::Request, regex_manager: &mut RegexManager) -> bool;

    #[cfg(test)]
    fn matches_test(&self, request: &request::Request) -> bool;
}

impl NetworkMatchable for NetworkFilter {
    fn matches(&self, request: &request::Request, regex_manager: &mut RegexManager) -> bool {
        check_options(self, request) && check_pattern(self, request, regex_manager)
    }

    #[cfg(test)]
    fn matches_test(&self, request: &request::Request) -> bool {
        self.matches(request, &mut RegexManager::default())
    }
}

// ---------------------------------------------------------------------------
// Filter parsing
// ---------------------------------------------------------------------------

fn compute_filter_id(
    modifier_option: Option<&str>,
    mask: NetworkFilterMask,
    filter: Option<&str>,
    hostname: Option<&str>,
    opt_domains: Option<&Vec<Hash>>,
    opt_not_domains: Option<&Vec<Hash>>,
) -> Hash {
    let mut hash: Hash = (5408 * 33) ^ Hash::from(mask.bits);

    if let Some(s) = modifier_option {
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

/// Compiles a filter pattern to a regex. This is only performed *lazily* for
/// filters containing at least a * or ^ symbol. Because Regexes are expansive,
/// we try to convert some patterns to plain filters.
#[allow(clippy::trivial_regex)]
pub(crate) fn compile_regex(
    filter: &FilterPart,
    is_right_anchor: bool,
    is_left_anchor: bool,
    is_complete_regex: bool,
) -> CompiledRegex {
    // Escape special regex characters: |.$+?{}()[]\
    static SPECIAL_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"([\|\.\$\+\?\{\}\(\)\[\]])").unwrap());
    // * can match anything
    static WILDCARD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*").unwrap());
    // ^ can match any separator or the end of the pattern
    static ANCHOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\^(.)").unwrap());
    // ^ can match any separator or the end of the pattern
    static ANCHOR_RE_EOL: Lazy<Regex> = Lazy::new(|| Regex::new(r"\^$").unwrap());

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
        match BytesRegexBuilder::new(pattern).unicode(false).build() {
            Ok(compiled) => CompiledRegex::Compiled(compiled),
            Err(e) => {
                // println!("Regex parsing failed ({:?})", e);
                CompiledRegex::RegexParsingError(e)
            }
        }
    } else {
        match BytesRegexSetBuilder::new(escaped_patterns)
            .unicode(false)
            .build()
        {
            Ok(compiled) => CompiledRegex::CompiledSet(compiled),
            Err(e) => CompiledRegex::RegexParsingError(e),
        }
    }
}

/// Check if the sub-string contained between the indices start and end is a
/// regex filter (it contains a '*' or '^' char). Here we are limited by the
/// capability of javascript to check the presence of a pattern between two
/// indices (same for Regex...).
fn check_is_regex(filter: &str) -> bool {
    // TODO - we could use sticky regex here
    let start_index = find_char(b'*', filter.as_bytes());
    let separator_index = find_char(b'^', filter.as_bytes());
    start_index.is_some() || separator_index.is_some()
}

/// Handle hostname anchored filters, given 'hostname' from ||hostname and
/// request's hostname, check if there is a match. This is tricky because
/// filters authors rely and different assumption. We can have prefix of suffix
/// matches of anchor.
fn is_anchored_by_hostname(
    filter_hostname: &str,
    hostname: &str,
    wildcard_filter_hostname: bool,
) -> bool {
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
    } else if let Some(match_index) = memmem::find(hostname.as_bytes(), filter_hostname.as_bytes())
    {
        if match_index == 0 {
            // `filter_hostname` is a prefix of `hostname` and needs to match full a label.
            //
            // Examples (filter_hostname, hostname):
            //   * (foo, foo.com)
            //   * (sub.foo, sub.foo.com)
            wildcard_filter_hostname
                || filter_hostname.ends_with('.')
                || hostname[filter_hostname_len..].starts_with('.')
        } else if match_index == hostname_len - filter_hostname_len {
            // `filter_hostname` is a suffix of `hostname`.
            //
            // Examples (filter_hostname, hostname):
            //    * (foo.com, sub.foo.com)
            //    * (com, foo.com)
            filter_hostname.starts_with('.') || hostname[match_index - 1..].starts_with('.')
        } else {
            // `filter_hostname` is infix of `hostname` and needs match full labels
            (wildcard_filter_hostname
                || filter_hostname.ends_with('.')
                || hostname[filter_hostname_len..].starts_with('.'))
                && (filter_hostname.starts_with('.')
                    || hostname[match_index - 1..].starts_with('.'))
        }
    } else {
        // No match
        false
    }
}

fn get_url_after_hostname<'a>(url: &'a str, hostname: &str) -> &'a str {
    let start =
        memmem::find(url.as_bytes(), hostname.as_bytes()).unwrap_or(url.len() - hostname.len());
    &url[start + hostname.len()..]
}

// ---------------------------------------------------------------------------
// Filter matching
// ---------------------------------------------------------------------------

// pattern
fn check_pattern_plain_filter_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    let request_url = request.get_url(filter.match_case());
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => memmem::find(request_url.as_bytes(), f.as_bytes()).is_some(),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if memmem::find(request_url.as_bytes(), f.as_bytes()).is_some() {
                    return true;
                }
            }
            false
        }
    }
}

// pattern|
fn check_pattern_right_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    let request_url = request.get_url(filter.match_case());
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => request_url.ends_with(f),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if request_url.ends_with(f) {
                    return true;
                }
            }
            false
        }
    }
}

// |pattern
fn check_pattern_left_anchor_filter(filter: &NetworkFilter, request: &request::Request) -> bool {
    let request_url = request.get_url(filter.match_case());
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => request_url.starts_with(f),
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if request_url.starts_with(f) {
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
    let request_url = request.get_url(filter.match_case());
    match &filter.filter {
        FilterPart::Empty => true,
        FilterPart::Simple(f) => &request_url == f,
        FilterPart::AnyOf(filters) => {
            for f in filters {
                if &request_url == f {
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
    regex_manager: &mut RegexManager,
) -> bool {
    let request_url = request.get_url(filter.match_case());
    regex_manager.matches(filter, &request_url[start_from..])
}

fn check_pattern_regex_filter(
    filter: &NetworkFilter,
    request: &request::Request,
    regex_manager: &mut RegexManager,
) -> bool {
    check_pattern_regex_filter_at(filter, request, 0, regex_manager)
}

// ||pattern*^
fn check_pattern_hostname_anchor_regex_filter(
    filter: &NetworkFilter,
    request: &request::Request,
    regex_manager: &mut RegexManager,
) -> bool {
    let request_url = request.get_url(filter.match_case());
    filter
        .hostname
        .as_ref()
        .map(|hostname| {
            if is_anchored_by_hostname(
                hostname,
                &request.hostname,
                filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX),
            ) {
                check_pattern_regex_filter_at(
                    filter,
                    request,
                    memmem::find(request_url.as_bytes(), hostname.as_bytes()).unwrap_or_default()
                        + hostname.len(),
                    regex_manager,
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
            if is_anchored_by_hostname(
                hostname,
                &request.hostname,
                filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX),
            ) {
                match &filter.filter {
                    // In this specific case it means that the specified hostname should match
                    // at the end of the hostname of the request. This allows to prevent false
                    // positive like ||foo.bar which would match https://foo.bar.baz where
                    // ||foo.bar^ would not.
                    FilterPart::Empty => {
                        request.hostname.len() == hostname.len()        // if lengths are equal, hostname equality is implied by anchoring check
                            || request.hostname.ends_with(hostname)
                    }
                    _ => check_pattern_right_anchor_filter(filter, request),
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
            if is_anchored_by_hostname(
                hostname,
                &request.hostname,
                filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX),
            ) {
                let request_url = request.get_url(filter.match_case());
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Since it must follow immediatly after the hostname and be a suffix of
                    // the URL, we conclude that filter must be equal to the part of the
                    // url following the hostname.
                    FilterPart::Simple(f) => get_url_after_hostname(&request_url, hostname) == f,
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request_url, hostname);
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
            if is_anchored_by_hostname(
                hostname,
                &request.hostname,
                filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX),
            ) {
                let request_url = request.get_url(filter.match_case());
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Since this is not a regex, the filter pattern must follow the hostname
                    // with nothing in between. So we extract the part of the URL following
                    // after hostname and will perform the matching on it.
                    FilterPart::Simple(f) => {
                        get_url_after_hostname(&request_url, hostname).starts_with(f)
                    }
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request_url, hostname);
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
            if is_anchored_by_hostname(
                hostname,
                &request.hostname,
                filter.mask.contains(NetworkFilterMask::IS_HOSTNAME_REGEX),
            ) {
                let request_url = request.get_url(filter.match_case());
                match &filter.filter {
                    // if no filter, we have a match
                    FilterPart::Empty => true,
                    // Filter hostname does not necessarily have to be a full, proper hostname, part of it can be lumped together with the URL
                    FilterPart::Simple(f) => {
                        get_url_after_hostname(&request_url, hostname).contains(f)
                    }
                    FilterPart::AnyOf(filters) => {
                        let url_after_hostname = get_url_after_hostname(&request_url, hostname);
                        for f in filters {
                            if url_after_hostname.contains(f) {
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

/// Efficiently checks if a certain network filter matches against a network
/// request.
fn check_pattern(
    filter: &NetworkFilter,
    request: &request::Request,
    regex_manager: &mut RegexManager,
) -> bool {
    if filter.is_hostname_anchor() {
        if filter.is_regex() {
            check_pattern_hostname_anchor_regex_filter(filter, request, regex_manager)
        } else if filter.is_right_anchor() && filter.is_left_anchor() {
            check_pattern_hostname_left_right_anchor_filter(filter, request)
        } else if filter.is_right_anchor() {
            check_pattern_hostname_right_anchor_filter(filter, request)
        } else if filter.is_left_anchor() {
            check_pattern_hostname_left_anchor_filter(filter, request)
        } else {
            check_pattern_hostname_anchor_filter(filter, request)
        }
    } else if filter.is_regex() || filter.is_complete_regex() {
        check_pattern_regex_filter(filter, request, regex_manager)
    } else if filter.is_left_anchor() && filter.is_right_anchor() {
        check_pattern_left_right_anchor_filter(filter, request)
    } else if filter.is_left_anchor() {
        check_pattern_left_anchor_filter(filter, request)
    } else if filter.is_right_anchor() {
        check_pattern_right_anchor_filter(filter, request)
    } else {
        check_pattern_plain_filter_filter(filter, request)
    }
}

fn check_options(filter: &NetworkFilter, request: &request::Request) -> bool {
    // Bad filter never matches
    if filter.is_badfilter() {
        return false;
    }
    // We first discard requests based on type, protocol and party. This is really
    // cheap and should be done first.
    if !filter.check_cpt_allowed(&request.request_type)
        || (request.is_https && !filter.for_https())
        || (request.is_http && !filter.for_http())
        || (!filter.first_party() && !request.is_third_party)
        || (!filter.third_party() && request.is_third_party)
    {
        return false;
    }

    // Source URL must be among these domains to match
    if let Some(included_domains) = filter.opt_domains.as_ref() {
        if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
            // If the union of included domains is recorded
            if let Some(included_domains_union) = filter.opt_domains_union {
                // If there isn't any source hash that matches the union, there's no match at all
                if source_hashes
                    .iter()
                    .all(|h| h & included_domains_union != *h)
                {
                    return false;
                }
            }
            if source_hashes
                .iter()
                .all(|h| !utils::bin_lookup(included_domains, *h))
            {
                return false;
            }
        }
    }

    if let Some(excluded_domains) = filter.opt_not_domains.as_ref() {
        if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
            // If the union of excluded domains is recorded
            if let Some(excluded_domains_union) = filter.opt_not_domains_union {
                // If there's any source hash that matches the union, check the actual values
                if source_hashes.iter().any(|h| {
                    (h & excluded_domains_union == *h) && utils::bin_lookup(excluded_domains, *h)
                }) {
                    return false;
                }
            } else if source_hashes
                .iter()
                .any(|h| utils::bin_lookup(excluded_domains, *h))
            {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
#[path = "../../tests/unit/filters/network.rs"]
mod unit_tests;
