use crate::filters::network::{NetworkFilter, NetworkFilterError};
use crate::filters::cosmetic::{CosmeticFilter, CosmeticFilterError};
use itertools::Either;

use itertools::Itertools;

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
    pub fn add_filter_list(&mut self, filter_list: &str) {
        let rules = filter_list.lines().map(str::to_string).collect::<Vec<_>>();
        self.add_filters(&rules);
    }

    /// Adds a collection of filter rules to this `FilterSet`. Filters that cannot be parsed
    /// successfully are ignored.
    pub fn add_filters(&mut self, filters: &[String]) {
        let (mut parsed_network_filters, mut parsed_cosmetic_filters) = parse_filters(&filters, self.debug);
        self.network_filters.append(&mut parsed_network_filters);
        self.cosmetic_filters.append(&mut parsed_cosmetic_filters);
    }

    /// Adds the string representation of a single filter rule to this `FilterSet`.
    pub fn add_filter(&mut self, filter: &str) -> Result<(), FilterParseError> {
        let filter_parsed = parse_filter(filter, self.debug);
        match filter_parsed? {
            ParsedFilter::Network(filter) => self.network_filters.push(filter),
            ParsedFilter::Cosmetic(filter) => self.cosmetic_filters.push(filter),
        }
        Ok(())
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
    debug: bool
) -> Result<ParsedFilter, FilterParseError> {

    let filter = line.trim();

    if filter.is_empty() {
        return Err(FilterParseError::Empty);
    }

    match detect_filter_type(filter) {
        FilterType::Network => NetworkFilter::parse(filter, debug)
            .map(|f| f.into())
            .map_err(|e| e.into()),
        FilterType::Cosmetic => CosmeticFilter::parse(filter, debug)
            .map(|f| f.into())
            .map_err(|e| e.into()),
        _ => Err(FilterParseError::Unsupported),
    }
}

/// Parse an entire list of filters, ignoring any errors
pub fn parse_filters(
    list: &[String],
    debug: bool,
) -> (Vec<NetworkFilter>, Vec<CosmeticFilter>) {

    let list_iter = list.iter();

    let (network_filters, cosmetic_filters): (Vec<_>, Vec<_>) = list_iter
        .map(|line| parse_filter(line, debug))
        .filter_map(Result::ok)
        .partition_map(|filter| match filter {
            ParsedFilter::Network(f) => Either::Left(f),
            ParsedFilter::Cosmetic(f) => Either::Right(f),
        });

    (network_filters, cosmetic_filters)
}

/**
 * Given a single line (string), checks if this would likely be a cosmetic
 * filter, a network filter or something that is not supported. This check is
 * performed before calling a more specific parser to create an instance of
 * `NetworkFilter` or `CosmeticFilter`.
 */
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
    if filter.find("$$").is_some() {
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
    fn parse_filter_failed_fuzz_1() {
        let input = "Ѥ";
        let result = parse_filter(input, true);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_2() {
        assert!(parse_filter(r#"###\\\00DB \008D"#, true).is_ok());
        assert!(parse_filter(r#"###\Û"#, true).is_ok());
    }

    #[test]
    fn parse_filter_failed_fuzz_3() {
        let input = "||$3p=/";
        let result = parse_filter(input, true);
        assert!(result.is_ok());
    }
    
    #[test]
    fn parse_filter_failed_fuzz_4() {
        // \\##+js(,\xdd\x8d
        assert!(parse_filter(
            &String::from_utf8(vec![92, 35, 35, 43, 106, 115, 40, 44, 221, 141]).unwrap(),
            true,
        ).is_ok());
    }
    
}
