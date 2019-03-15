use crate::filters::network::NetworkFilter;
use itertools::{Either};
use rayon::prelude::*;

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
    ParseError
}

pub fn parse_filters(
    list: &Vec<String>,
    load_network_filters: bool,
    load_cosmetic_filters: bool,
    debug: bool,
) -> (Vec<NetworkFilter>, Vec<String>) {
    // let mut network_filters = Vec::with_capacity(list.len());
    // let cosmetic_filters = vec![];

    let (network_filters, cosmetic_filters): (Vec<_>, Vec<_>) = list
    .into_par_iter()
    .map(|line| {
        let filter = line.trim();
        if filter.len() > 0 {
            let filter_type = detect_filter_type(filter);
            if filter_type == FilterType::Network && load_network_filters {
                let network_filter = NetworkFilter::parse(filter, debug);
                let res: Result<Either<NetworkFilter, String>, FilterError> = network_filter.map(|f| Either::Left(f)).or_else(|_| Err(FilterError::ParseError));
                res
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
    .partition_map(|filter| {
        match filter {
            Either::Left(f) => Either::Left(f),
            Either::Right(f) => Either::Right(f)
        }
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
    let sharp_index = filter.find('#');
    if sharp_index.is_some() {
        let after_sharp_index = sharp_index.unwrap() + 1;

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
        } else if filter[after_sharp_index..].starts_with(/* ## */ "#")
            || filter[after_sharp_index..].starts_with(/* #@# */ "@#")
        {
            // Parse supported cosmetic filter
            // `##` `#@#`
            return FilterType::Cosmetic;
        }
    }

    // Everything else is a network filter
    return FilterType::Network;
}
