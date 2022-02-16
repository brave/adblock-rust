//! Parsing functions and collections for handling with multiple filter rules.

use crate::filters::network::{NetworkFilter, NetworkFilterError};
use crate::filters::cosmetic::{CosmeticFilter, CosmeticFilterError};

use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};

/// Specifies rule types to keep during parsing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RuleTypes {
    All,
    NetworkOnly,
    CosmeticOnly,
}

impl Default for RuleTypes {
    fn default() -> Self {
        Self::All
    }
}

impl RuleTypes {
    fn loads_network_rules(&self) -> bool {
        match self {
            Self::All => true,
            Self::NetworkOnly => true,
            _ => false,
        }
    }

    fn loads_cosmetic_rules(&self) -> bool {
        match self {
            Self::All => true,
            Self::CosmeticOnly => true,
            _ => false,
        }
    }
}

/// Options for tweaking how a filter or list of filters is interpreted when parsing. It's
/// recommended to use _struct update syntax_ with a `default()` "rest" value; adding new fields to
/// this struct will not be considered a breaking change.
///
/// ```
/// # use adblock::lists::{FilterFormat, ParseOptions};
/// let parse_options = ParseOptions {
///     format: FilterFormat::Hosts,
///     ..ParseOptions::default()
/// };
/// ```
#[derive(Copy, Clone, Deserialize)]
pub struct ParseOptions {
    /// Assume filters are in the given format when parsing. Defaults to `FilterFormat::Standard`.
    #[serde(default)]
    pub format: FilterFormat,
    /// The `$redirect-url` filter option can redirect to an arbitrary HTTP/HTTPS resource over the
    /// network. By default this is disabled for security concerns, and any rule containing a
    /// `redirect-url` option will be ignored.
    #[serde(default)]
    pub include_redirect_urls: bool,
    /// Specifies rule types to keep during parsing. Defaults to `RuleTypes::All`. This can be used
    /// to reduce the memory impact of engines that will only be used for cosmetic filtering or
    /// network filtering, but not both. It can also be useful for iOS and macOS when exporting to
    /// content-blocking syntax, as these platforms limit the number of content blocking rules that
    /// can be loaded.
    #[serde(default)]
    pub rule_types: RuleTypes,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            format: FilterFormat::Standard,
            include_redirect_urls: false,
            rule_types: RuleTypes::All,
        }
    }
}

/// Manages a set of rules to be added to an `Engine`.
///
/// To be able to efficiently handle special options like `$badfilter`, and to allow optimizations,
/// all rules must be available when the `Engine` is first created. `FilterSet` allows assembling a
/// compound list from multiple different sources before compiling the rules into an `Engine`.
#[derive(Clone)]
pub struct FilterSet {
    debug: bool,
    pub(crate) network_filters: Vec<NetworkFilter>,
    pub(crate) cosmetic_filters: Vec<CosmeticFilter>,
}

impl Default for FilterSet {
    /// Equivalent to `FilterSet::new(false)`, or `FilterSet::new(true)` when compiled in test
    /// configuration.
    fn default() -> Self {
        #[cfg(not(test))]
        let debug = false;

        #[cfg(test)]
        let debug = true;

        Self::new(debug)
    }
}

impl FilterSet {
    /// Creates a new `FilterSet`. `debug` specifies whether or not to save information about the
    /// original raw filter rules alongside the more compact internal representation. If enabled,
    /// this information will be passed to the corresponding `Engine`.
    pub fn new(debug: bool) -> Self {
        Self {
            debug,
            network_filters: Vec::new(),
            cosmetic_filters: Vec::new(),
        }
    }

    /// Adds the contents of an entire filter list to this `FilterSet`. Filters that cannot be
    /// parsed successfully are ignored.
    pub fn add_filter_list(&mut self, filter_list: &str, opts: ParseOptions) {
        let rules = filter_list.lines().map(str::to_string).collect::<Vec<_>>();
        self.add_filters(&rules, opts);
    }

    /// Adds a collection of filter rules to this `FilterSet`. Filters that cannot be parsed
    /// successfully are ignored.
    pub fn add_filters(&mut self, filters: &[String], opts: ParseOptions) {
        let (mut parsed_network_filters, mut parsed_cosmetic_filters) = parse_filters(&filters, self.debug, opts);
        self.network_filters.append(&mut parsed_network_filters);
        self.cosmetic_filters.append(&mut parsed_cosmetic_filters);
    }

    /// Adds the string representation of a single filter rule to this `FilterSet`.
    pub fn add_filter(&mut self, filter: &str, opts: ParseOptions) -> Result<(), FilterParseError> {
        let filter_parsed = parse_filter(filter, self.debug, opts);
        match filter_parsed? {
            ParsedFilter::Network(filter) => self.network_filters.push(filter),
            ParsedFilter::Cosmetic(filter) => self.cosmetic_filters.push(filter),
        }
        Ok(())
    }

    /// Consumes this `FilterSet`, returning an equivalent list of content blocking rules and a
    /// corresponding new list containing the `String` representation of all filters that were
    /// successfully converted (as `FilterFormat::Standard` rules).
    ///
    /// The list of content blocking rules will be properly ordered to ensure correct behavior of
    /// `ignore-previous-rules`-typed rules.
    ///
    /// This function will fail if the `FilterSet` was not created in debug mode.
    #[cfg(feature = "content-blocking")]
    pub fn into_content_blocking(self) -> Result<(Vec<crate::content_blocking::CbRule>, Vec<String>), ()> {
        use std::convert::TryInto;
        use crate::content_blocking;

        if !self.debug {
            return Err(())
        }

        let mut ignore_previous_rules = vec![];
        let mut other_rules = vec![];

        let mut filters_used = vec![];

        self.network_filters.into_iter().for_each(|filter| {
            let original_rule = filter.raw_line.clone().expect("All rules should be in debug mode");
            if let Ok(equivalent) = TryInto::<content_blocking::CbRuleEquivalent>::try_into(filter) {
                filters_used.push(original_rule);
                equivalent.into_iter().for_each(|cb_rule| {
                    match &cb_rule.action.typ {
                        content_blocking::CbType::IgnorePreviousRules => ignore_previous_rules.push(cb_rule),
                        _ => other_rules.push(cb_rule),
                    }
                });
            }
        });

        let add_fp_document_exception = !filters_used.is_empty();

        self.cosmetic_filters.into_iter().for_each(|filter| {
            let original_rule = filter.raw_line.clone().expect("All rules should be in debug mode");
            if let Ok(cb_rule) = TryInto::<content_blocking::CbRule>::try_into(filter) {
                filters_used.push(original_rule);
                match &cb_rule.action.typ {
                    content_blocking::CbType::IgnorePreviousRules => ignore_previous_rules.push(cb_rule),
                    _ => other_rules.push(cb_rule),
                }
            }
        });

        other_rules.append(&mut ignore_previous_rules);

        if add_fp_document_exception {
            other_rules.push(content_blocking::ignore_previous_fp_documents());
        }

        Ok((other_rules, filters_used))
    }
}

/// Denotes the format of a particular list resource, which affects how its rules should be parsed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FilterFormat {
    /// Rules should be parsed in ABP/uBO-style format.
    Standard,
    /// Each line consists of an IP address (usually 127.0.0.1 or 0.0.0.0), some whitespace, and a
    /// single hostname. This syntax is normally used directly for HOSTS-based adblockers. These
    /// rules will be treated equivalently to `"||hostname^"` rules in `Standard` format; the IP
    /// addresses will not be used.
    ///
    /// Note that some sources provide a more raw format, where each line consists of just a
    /// hostname. This option will also accept that format.
    ///
    /// For this option, `!` is accepted as a comment character at the beginning of a line, and `#`
    /// is accepted as a comment character anywhere in a line.
    Hosts,
}

/// Default to parsing lists in `Standard` format.
impl Default for FilterFormat {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, PartialEq)]
pub enum FilterType {
    Network,
    Cosmetic,
    NotSupported,
}

/// Successful result of parsing a single filter rule
pub enum ParsedFilter {
    Network(NetworkFilter),
    Cosmetic(CosmeticFilter),
}

impl From<NetworkFilter> for ParsedFilter {
    fn from(v: NetworkFilter) -> Self {
        ParsedFilter::Network(v)
    }
}

impl From<CosmeticFilter> for ParsedFilter {
    fn from(v: CosmeticFilter) -> Self {
        ParsedFilter::Cosmetic(v)
    }
}

/// Unsuccessful result of parsing a single filter rule.
#[derive(Debug)]
pub enum FilterParseError {
    Network(NetworkFilterError),
    Cosmetic(CosmeticFilterError),
    Unsupported,
    Empty,
}

impl From<NetworkFilterError> for FilterParseError {
    fn from(v: NetworkFilterError) -> Self {
        FilterParseError::Network(v)
    }
}

impl From<CosmeticFilterError> for FilterParseError {
    fn from(v: CosmeticFilterError) -> Self {
        FilterParseError::Cosmetic(v)
    }
}

/// Parse a single filter rule
pub fn parse_filter(
    line: &str,
    debug: bool,
    opts: ParseOptions,
) -> Result<ParsedFilter, FilterParseError> {
    let filter = line.trim();

    if filter.is_empty() {
        return Err(FilterParseError::Empty);
    }

    match opts.format {
        FilterFormat::Standard => {
            match (detect_filter_type(filter), opts.rule_types) {
                (FilterType::Network, RuleTypes::All | RuleTypes::NetworkOnly) => NetworkFilter::parse(filter, debug, opts)
                    .map(|f| f.into())
                    .map_err(|e| e.into()),
                (FilterType::Cosmetic, RuleTypes::All | RuleTypes::CosmeticOnly) => CosmeticFilter::parse(filter, debug)
                    .map(|f| f.into())
                    .map_err(|e| e.into()),
                _ => Err(FilterParseError::Unsupported),
            }
        }
        FilterFormat::Hosts => {
            // Hosts-style rules can only ever be network rules
            if !opts.rule_types.loads_network_rules() {
                return Err(FilterParseError::Unsupported);
            }
            if filter.starts_with('!') {
                return Err(FilterParseError::Unsupported);
            }
            // Discard contents after first `#` character
            let filter = if let Some(hash_loc) = filter.find('#') {
                let filter = &filter[..hash_loc];
                let filter = filter.trim();

                if filter.is_empty() {
                    return Err(FilterParseError::Unsupported);
                }

                filter
            } else {
                filter
            };

            // Take the last of at most 2 whitespace separated fields
            let mut filter_parts = filter.split_whitespace();
            let hostname = match (filter_parts.next(), filter_parts.next(), filter_parts.next()) {
                (None, None, None) => return Err(FilterParseError::Unsupported),
                (Some(hostname), None, None) => hostname,
                (Some(_ip), Some(hostname), None) => hostname,
                (Some(_), Some(_), Some(_)) => return Err(FilterParseError::Unsupported),
                _ => unreachable!(),
            };

            // Matches in hosts lists are usually redirected to localhost. For that reason, some
            // lists include an entry for "localhost", which should be explicitly ignored when
            // performing request-level adblocking.
            if hostname == "localhost" {
                return Err(FilterParseError::Unsupported);
            }

            NetworkFilter::parse_hosts_style(hostname, debug)
                .map(|f| f.into())
                .map_err(|e| e.into())
        }
    }
}

/// Parse an entire list of filters, ignoring any errors
pub fn parse_filters(
    list: &[String],
    debug: bool,
    opts: ParseOptions,
) -> (Vec<NetworkFilter>, Vec<CosmeticFilter>) {
    let list_iter = list.iter();

    let (network_filters, cosmetic_filters): (Vec<_>, Vec<_>) = list_iter
        .map(|line| parse_filter(line, debug, opts))
        .filter_map(Result::ok)
        .partition_map(|filter| match filter {
            ParsedFilter::Network(f) => Either::Left(f),
            ParsedFilter::Cosmetic(f) => Either::Right(f),
        });

    (network_filters, cosmetic_filters)
}

/// Given a single line, checks if this would likely be a cosmetic filter, a
/// network filter or something that is not supported. This check is performed
/// before calling a more specific parser to create an instance of
/// `NetworkFilter` or `CosmeticFilter`.
fn detect_filter_type(filter: &str) -> FilterType {
    // Ignore comments
    if filter.len() == 1
        || filter.starts_with('!')
        || (filter.starts_with('#') && filter[1..].starts_with(char::is_whitespace))
        || filter.starts_with("[Adblock")
    {
        return FilterType::NotSupported;
    }

    if filter.starts_with('|') || filter.starts_with("@@|") {
        return FilterType::Network;
    }

    // Ignore Adguard cosmetics
    // `$$`
    if filter.contains("$$") {
        return FilterType::NotSupported;
    }

    // Check if filter is cosmetics
    if let Some(sharp_index) = filter.find('#') {
        let after_sharp_index = sharp_index + 1;

        // Ignore Adguard cosmetics
        // `#$#` `#@$#`
        // `#%#` `#@%#`
        // `#?#`
        if filter[after_sharp_index..].starts_with(/* #@$# */ "@$#")
            || filter[after_sharp_index..].starts_with(/* #@%# */ "@%#")
            || filter[after_sharp_index..].starts_with(/* #%# */ "%#")
            || filter[after_sharp_index..].starts_with(/* #$# */ "$#")
            || filter[after_sharp_index..].starts_with(/* #?# */ "?#")
        {
            return FilterType::NotSupported;
        } else if filter[after_sharp_index..].starts_with(/* ## */ '#')
            || filter[after_sharp_index..].starts_with(/* #@# */ "@#")
        {
            // Parse supported cosmetic filter
            // `##` `#@#`
            return FilterType::Cosmetic;
        }
    }

    // Everything else is a network filter
    FilterType::Network
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hosts_style() {
        {
            let input = "www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "www.malware.com/virus.txt";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "127.0.0.1\t\twww.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "0.0.0.0    www.malware.com     # replace after issue #289336 is addressed";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_ok());
        }
        {
            let input = "! Title: list.txt";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 localhost";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "127.0.0.1 com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = ".com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "*.com";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
        {
            let input = "www.";
            let result = parse_filter(input, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
            assert!(result.is_err());
        }
    }

    #[test]
    fn parse_filter_failed_fuzz_1() {
        let input = "Ѥ";
        let result = parse_filter(input, true, Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_2() {
        assert!(parse_filter(r#"###\\\00DB \008D"#, true, Default::default()).is_ok());
        assert!(parse_filter(r#"###\Û"#, true, Default::default()).is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_3() {
        let input = "||$3p=/";
        let result = parse_filter(input, true, Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_4() {
        // \\##+js(,\xdd\x8d
        let parsed = parse_filter(
            &String::from_utf8(vec![92, 35, 35, 43, 106, 115, 40, 44, 221, 141]).unwrap(),
            true,
            Default::default(),
        );
        #[cfg(feature = "css-validation")]
        assert!(parsed.is_err());
        #[cfg(not(feature = "css-validation"))]
        assert!(parsed.is_ok());
    }

    #[test]
    #[cfg(feature = "css-validation")]
    fn parse_filter_opening_comment() {
        assert!(parse_filter(
            "##input,input/*",
            true,
            Default::default(),
        ).is_err());
    }
}
