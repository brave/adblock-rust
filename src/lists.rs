use crate::filters::network::NetworkFilter;
use itertools::Either;
use serde::{Serialize, Deserialize};

use itertools::Itertools;

#[derive(Debug, PartialEq)]
pub enum FilterType {
    Network,
    Cosmetic,
    NotSupported,
}

#[derive(Debug, PartialEq)]
pub enum FilterError {
    NotSupported,
    NotImplemented,
    Empty,
    ParseError,
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

pub fn parse_filters(
    list: &[String],
    load_network_filters: bool,
    load_cosmetic_filters: bool,
    debug: bool,
) -> (Vec<NetworkFilter>, Vec<String>) {

    let list_iter = list.iter();

    let (network_filters, cosmetic_filters): (Vec<_>, Vec<_>) = list_iter
        .map(|line| {
            let filter = line.trim();
            if !filter.is_empty() {
                let filter_type = detect_filter_type(filter);
                if filter_type == FilterType::Network && load_network_filters {
                    let network_filter = NetworkFilter::parse(filter, debug);
                    // if debug && network_filter.is_err() {
                    //     println!("Error parsing rule {}: {:?}", filter, network_filter.as_ref().err())
                    // }
                    network_filter
                        .map(Either::Left)
                        .or_else(|_| Err(FilterError::ParseError))
                } else if filter_type == FilterType::Cosmetic && load_cosmetic_filters {
                    // TODO: unimplemented, just return rule as a string
                    Ok(Either::Right(String::from(filter)))
                } else {
                    Err(FilterError::NotSupported)
                }
            } else {
                Err(FilterError::Empty)
            }
        })
        .filter_map(Result::ok)
        .partition_map(|filter| match filter {
            Either::Left(f) => Either::Left(f),
            Either::Right(f) => Either::Right(f),
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
