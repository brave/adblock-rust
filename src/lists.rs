use crate::filters::network::{NetworkFilter, NetworkFilterError};
use crate::filters::cosmetic::{CosmeticFilter, CosmeticFilterError};
use itertools::Either;
use serde::{Serialize, Deserialize};

use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum FilterType {
    Network,
    Cosmetic,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterList {
    pub uuid: String,
    pub url: String,
    pub title: String,
    pub langs: Vec<String>,
    pub support_url: String,
    pub component_id: String,
    pub base64_public_key: String,
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
/// Unused means that e.g. only network filters were requested, but a cosmetic filter was
/// specified.
pub enum FilterParseError {
    Network(NetworkFilterError),
    Cosmetic(CosmeticFilterError),
    Unsupported,
    Unused,
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
    load_network_filters: bool,
    load_cosmetic_filters: bool,
    debug: bool
) -> Result<ParsedFilter, FilterParseError> {

    let filter = line.trim();

    if filter.is_empty() {
        return Err(FilterParseError::Empty);
    }

    match detect_filter_type(filter) {
        FilterType::Network => {
            if load_network_filters {
                NetworkFilter::parse(filter, debug)
                    .map(|f| f.into())
                    .map_err(|e| e.into())
            } else {
                Err(FilterParseError::Unused)
            }
        }
        FilterType::Cosmetic => {
            if load_cosmetic_filters {
                CosmeticFilter::parse(filter, debug)
                    .map(|f| f.into())
                    .map_err(|e| e.into())
            } else {
                Err(FilterParseError::Unused)
            }
        }
        _ => Err(FilterParseError::Unsupported),
    }
}

/// Parse an entire list of filters, ignoring any errors
pub fn parse_filters(
    list: &[String],
    load_network_filters: bool,
    load_cosmetic_filters: bool,
    debug: bool,
) -> (Vec<NetworkFilter>, Vec<CosmeticFilter>) {

    let list_iter = list.iter();

    let (network_filters, cosmetic_filters): (Vec<_>, Vec<_>) = list_iter
        .map(|line| parse_filter(line, load_network_filters, load_cosmetic_filters, debug))
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
