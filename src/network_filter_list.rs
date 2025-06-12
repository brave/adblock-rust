//! Holds the implementation of [NetworkFilterList] and related functionality.

use std::{collections::HashMap, collections::HashSet, fmt};

use crate::filters::fb_network::flat::fb;
use crate::filters::fb_network::{FlatNetworkFilter, FlatNetworkFiltersListBuilder};
use crate::filters::flat_filter_map::FlatFilterMap;
use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable, TokenListType,
    TOKENS_BUFFER_CAPACITY,
};
use crate::filters::unsafe_tools::{fb_vector_to_slice, VerifiedFlatFilterListMemory};
use crate::optimizer;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::{fast_hash, to_short_hash, Hash, ShortHash};

/// Holds token data for all filters in a flat structure to minimize allocations
struct FilterTokenData {
    /// All tokens from all filters stored in a single flat vector
    tokens: Vec<Hash>,
    /// For each filter, stores (start_index, length, token_type) in the tokens vector
    filter_ranges: Vec<(usize, usize, TokenListType)>,
}

/// Holds relevant information from a single matchin gnetwork filter rule as a result of querying a
/// [NetworkFilterList] for a given request.
pub struct CheckResult {
    pub filter_mask: NetworkFilterMask,
    pub modifier_option: Option<String>,
    pub raw_line: Option<String>,
}

impl From<&NetworkFilter> for CheckResult {
    fn from(filter: &NetworkFilter) -> Self {
        Self {
            filter_mask: filter.mask,
            modifier_option: filter.modifier_option.clone(),
            raw_line: filter.raw_line.clone().map(|v| *v),
        }
    }
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(ref raw_line) = self.raw_line {
            write!(f, "{}", raw_line)
        } else {
            write!(f, "{}", self.filter_mask)
        }
    }
}

impl NetworkFilterMaskHelper for CheckResult {
    #[inline]
    fn has_flag(&self, v: NetworkFilterMask) -> bool {
        self.filter_mask.contains(v)
    }
}

#[derive(Debug, Clone)]
pub enum NetworkFilterListParsingError {
    InvalidFlatbuffer(flatbuffers::InvalidFlatbuffer),
    UniqueDomainsOutOfBounds(usize),
}

/// Internal structure to keep track of a collection of network filters.
pub(crate) struct NetworkFilterList {
    pub(crate) memory: VerifiedFlatFilterListMemory,
    pub(crate) unique_domains_hashes_map: HashMap<Hash, u32>,
}

impl Default for NetworkFilterList {
    fn default() -> Self {
        let mut builder = FlatNetworkFiltersListBuilder::new();
        let memory = builder.finish(HashMap::new());
        Self {
            memory,
            unique_domains_hashes_map: HashMap::new(),
        }
    }
}

/// Extracts tokens from all filters into a flat structure
fn extract_filter_tokens(filters: &[NetworkFilter]) -> FilterTokenData {
    let mut tokens = Vec::new();
    let mut filter_ranges = Vec::with_capacity(filters.len());
    let mut temp_tokens = Vec::with_capacity(TOKENS_BUFFER_CAPACITY);

    for filter in filters {
        temp_tokens.clear();
        let token_type = filter.get_tokens(&mut temp_tokens);

        let start_index = tokens.len();
        tokens.extend_from_slice(&temp_tokens);
        let length = temp_tokens.len();

        filter_ranges.push((start_index, length, token_type));
    }

    FilterTokenData {
        tokens,
        filter_ranges,
    }
}

/// Builds a token frequency histogram from flat token data
fn build_token_histogram(token_data: &FilterTokenData) -> (u32, HashMap<ShortHash, u32>) {
    let mut tokens_histogram: HashMap<ShortHash, u32> = HashMap::new();
    let mut total_number_of_tokens = 0u32;

    for &(start_index, length, _) in &token_data.filter_ranges {
        let filter_tokens = &token_data.tokens[start_index..start_index + length];
        for &token in filter_tokens {
            let short_token = to_short_hash(token);
            *tokens_histogram.entry(short_token).or_insert(0) += 1;
            total_number_of_tokens += 1;
        }
    }

    // Add bad tokens with high frequency to discourage their use
    for bad_token in ["http", "https", "www", "com"].iter() {
        tokens_histogram.insert(to_short_hash(fast_hash(bad_token)), total_number_of_tokens);
    }

    (total_number_of_tokens, tokens_histogram)
}

/// Finds the best (least frequent) token from a group of tokens
fn find_best_token(
    filter_tokens: &[Hash],
    tokens_histogram: &HashMap<ShortHash, u32>,
    total_number_of_tokens: u32,
) -> ShortHash {
    let mut best_token: ShortHash = 0;
    let mut min_count = total_number_of_tokens + 1;

    for &token in filter_tokens {
        let short_token = to_short_hash(token);
        match tokens_histogram.get(&short_token) {
            None => {
                return short_token; // Can't get better than 0
            }
            Some(&count) if count < min_count => {
                min_count = count;
                best_token = short_token;
            }
            _ => {}
        }
    }
    best_token
}

impl NetworkFilterList {
    /// Create a new [NetworkFilterList] from raw memory (includes verification).
    pub(crate) fn try_from_unverified_memory(
        flatbuffer_memory: Vec<u8>,
    ) -> Result<NetworkFilterList, NetworkFilterListParsingError> {
        let memory = VerifiedFlatFilterListMemory::from_raw(flatbuffer_memory)
            .map_err(NetworkFilterListParsingError::InvalidFlatbuffer)?;

        Self::try_from_verified_memory(memory)
    }

    pub(crate) fn try_from_verified_memory(
        memory: VerifiedFlatFilterListMemory,
    ) -> Result<NetworkFilterList, NetworkFilterListParsingError> {
        let root = memory.filter_list();

        // Reconstruct the unique_domains_hashes_map from the flatbuffer data
        let len = root.unique_domains_hashes().len();
        let mut unique_domains_hashes_map: HashMap<crate::utils::Hash, u32> =
            HashMap::with_capacity(len);
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            unique_domains_hashes_map.insert(
                hash,
                u32::try_from(index)
                    .map_err(|_| NetworkFilterListParsingError::UniqueDomainsOutOfBounds(index))?,
            );
        }

        Ok(Self {
            memory,
            unique_domains_hashes_map,
        })
    }

    pub fn get_filter_map(&self) -> FlatFilterMap<ShortHash, fb::NetworkFilter> {
        let filters_list = self.memory.filter_list();
        FlatFilterMap::new(
            fb_vector_to_slice(filters_list.filter_map_index()),
            filters_list.filter_map_values(),
        )
    }

    pub fn new(filters: Vec<NetworkFilter>, optimize: bool) -> Self {
        // Stage 1: Extract all tokens into a flat structure (single allocation)
        let token_data = extract_filter_tokens(&filters);

        // Stage 2: Build frequency histogram from flat token data
        let (total_number_of_tokens, tokens_histogram) = build_token_histogram(&token_data);

                // Stage 3: Create a single vector of (filter_index, token) tuples
        let mut filter_token_pairs = Vec::new();

        for (filter_index, _) in filters.iter().enumerate() {
            let (start_index, length, token_type) = {
                let range = &token_data.filter_ranges[filter_index];
                (range.0, range.1, range.2)
            };
            let filter_tokens = &token_data.tokens[start_index..start_index + length];

            match token_type {
                TokenListType::OptDomains => {
                    // For domain-based tokens, each domain is a separate bucket
                    for &token in filter_tokens {
                        let short_token = to_short_hash(token);
                        filter_token_pairs.push((filter_index, short_token));
                    }
                }
                TokenListType::AnyOf => {
                    // Find the best token for this filter
                    let best_token =
                        find_best_token(filter_tokens, &tokens_histogram, total_number_of_tokens);
                    filter_token_pairs.push((filter_index, best_token));
                }
            }
        }

        // Stage 4: Sort by token to group filters by token
        filter_token_pairs.sort_unstable_by_key(|(_, token)| *token);

        // Stage 5: Process filters - separate zero-token filters for optimization
        let mut token_filter_indices: Vec<(ShortHash, usize)> = Vec::new();
        let mut zero_token_indices: Vec<usize> = Vec::new();

        for (filter_index, token) in filter_token_pairs.drain(..) {
            if token != 0 {
                // Stage 5a: Collect filters with good tokens (use index, no cloning)
                token_filter_indices.push((token, filter_index));
            } else if optimize && optimizer::is_filter_optimizable_by_patterns(&filters[filter_index]) {
                // Stage 5b: Collect zero-token optimizable filter indices
                zero_token_indices.push(filter_index);
            } else {
                // Stage 5c: Non-optimizable zero-token filters
                token_filter_indices.push((0, filter_index));
            }
        }

        // Stage 6: Process zero-token optimizable filters through optimizer (only clone here)
        let mut optimized_filters = Vec::new();
        if !zero_token_indices.is_empty() {
            let zero_token_filters: Vec<NetworkFilter> = zero_token_indices.iter()
                .map(|&idx| filters[idx].clone())
                .collect();
            optimized_filters = optimizer::optimize(zero_token_filters);
        }

                // Stage 7: Build final flatbuffer structure - create builder only at the end
        let mut flat_builder = FlatNetworkFiltersListBuilder::new();
        let mut filter_map = HashMap::<ShortHash, Vec<u32>>::new();

        // Add all non-zero-token filters using indices (no cloning)
        for (token, filter_index) in token_filter_indices.drain(..) {
            let flat_index = flat_builder.add(&filters[filter_index]);
            filter_map.entry(token).or_default().push(flat_index);
        }

        // Add optimized zero-token filters (these were cloned for optimization)
        for filter in optimized_filters.drain(..) {
            let flat_index = flat_builder.add(&filter);
            filter_map.entry(0).or_default().push(flat_index);
        }

        // Stage 8: Finish building
        let memory = flat_builder.finish(filter_map);

        Self::try_from_verified_memory(memory).unwrap_or_default()
    }

    /// Returns the first found filter, if any, that matches the given request. The backing storage
    /// has a non-deterministic order, so this should be used for any category of filters where a
    /// match from each would be functionally equivalent. For example, if two different exception
    /// filters match a certain request, it doesn't matter _which_ one is matched - the request
    /// will be excepted either way.
    pub fn check(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Option<CheckResult> {
        let filters_list = self.memory.filter_list();

        if filters_list.filter_map_index().is_empty() {
            return None;
        }

        let filter_map = self.get_filter_map();

        for token in request.get_tokens_for_match() {
            for (index, fb_filter) in filter_map.get(to_short_hash(*token)) {
                let filter = FlatNetworkFilter::new(&fb_filter, index, self);

                // if matched, also needs to be tagged with an active tag (or not tagged at all)
                if filter.matches(request, regex_manager)
                    && filter.tag().is_none_or(|t| active_tags.contains(t))
                {
                    return Some(CheckResult {
                        filter_mask: filter.mask,
                        modifier_option: filter.modifier_option(),
                        raw_line: filter.raw_line(),
                    });
                }
            }
        }

        None
    }

    /// Returns _all_ filters that match the given request. This should be used for any category of
    /// filters where a match from each may carry unique information. For example, if two different
    /// `$csp` filters match a certain request, they may each carry a distinct CSP directive, and
    /// each directive should be combined for the final result.
    pub fn check_all(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Vec<CheckResult> {
        let mut filters: Vec<CheckResult> = vec![];

        let filters_list = self.memory.filter_list();

        if filters_list.filter_map_index().is_empty() {
            return filters;
        }

        let filter_map = self.get_filter_map();

        for token in request.get_tokens_for_match() {
            for (index, fb_filter) in filter_map.get(to_short_hash(*token)) {
                let filter = FlatNetworkFilter::new(&fb_filter, index, self);

                // if matched, also needs to be tagged with an active tag (or not tagged at all)
                if filter.matches(request, regex_manager)
                    && filter.tag().is_none_or(|t| active_tags.contains(t))
                {
                    filters.push(CheckResult {
                        filter_mask: filter.mask,
                        modifier_option: filter.modifier_option(),
                        raw_line: filter.raw_line(),
                    });
                }
            }
        }
        filters
    }
}

/// Inserts a value into the `Vec` under the specified key in the `HashMap`. The entry will be
/// created if it does not exist. If it already exists, it will be inserted in the `Vec` in a
/// sorted order.
pub(crate) fn insert_dup<K, V, H: std::hash::BuildHasher>(
    map: &mut HashMap<K, Vec<V>, H>,
    k: K,
    v: V,
) where
    K: std::cmp::Ord + std::hash::Hash,
    V: PartialOrd,
{
    let entry = map.entry(k).or_default();

    match entry.binary_search_by(|f| f.partial_cmp(&v).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(_pos) => (), // Can occur if the exact same rule is inserted twice. No reason to add anything.
        Err(slot) => entry.insert(slot, v),
    }
}
