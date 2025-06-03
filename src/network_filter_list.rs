use std::{collections::HashMap, collections::HashSet, fmt};

use crate::filters::fb_network::flat::fb;
use crate::filters::fb_network::{FlatNetworkFilter, FlatNetworkFiltersListBuilder};
use crate::filters::flat_filter_map::FlatFilterMap;
use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};
use crate::filters::unsafe_tools::{fb_vector_to_slice, VerifiedFlatFilterListMemory};
use crate::optimizer;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::{fast_hash, Hash};

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
pub enum FlatBufferParsingError {
    InvalidFlatbuffer(flatbuffers::InvalidFlatbuffer),
    UniqueDomainsOutOfBounds(usize),
}

pub(crate) struct NetworkFilterList {
    pub(crate) memory: VerifiedFlatFilterListMemory,
    pub(crate) unique_domains_hashes_map: HashMap<Hash, u16>,
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

impl NetworkFilterList {
    /// Create a new NetworkFilterList from raw memory (includes verification).
    pub(crate) fn try_from_unverified_memory(
        flatbuffer_memory: Vec<u8>,
    ) -> Result<NetworkFilterList, FlatBufferParsingError> {
        let memory = VerifiedFlatFilterListMemory::from_raw(flatbuffer_memory)
            .map_err(FlatBufferParsingError::InvalidFlatbuffer)?;

        Self::try_from_verified_memory(memory)
    }

    pub(crate) fn try_from_verified_memory(
        memory: VerifiedFlatFilterListMemory,
    ) -> Result<NetworkFilterList, FlatBufferParsingError> {
        let root = memory.filter_list();

        // Reconstruct the unique_domains_hashes_map from the flatbuffer data
        let len = root.unique_domains_hashes().len();
        let mut unique_domains_hashes_map: HashMap<crate::utils::Hash, u16> =
            HashMap::with_capacity(len);
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            unique_domains_hashes_map.insert(
                hash,
                u16::try_from(index)
                    .map_err(|_| FlatBufferParsingError::UniqueDomainsOutOfBounds(index))?,
            );
        }

        Ok(Self {
            memory,
            unique_domains_hashes_map,
        })
    }

    pub fn get_filter_map(&self) -> FlatFilterMap<Hash, fb::NetworkFilter> {
        let filters_list = self.memory.filter_list();
        FlatFilterMap::new(
            fb_vector_to_slice(filters_list.filter_map_index()),
            filters_list.filter_map_values(),
        )
    }

    pub fn new(filters: Vec<NetworkFilter>, optimize: bool) -> Self {
        // Compute tokens for all filters
        let filter_tokens: Vec<_> = filters
            .into_iter()
            .map(|filter| {
                let tokens = filter.get_tokens();
                (filter, tokens)
            })
            .collect();
        // compute the tokens' frequency histogram
        let (total_number_of_tokens, tokens_histogram) = token_histogram(&filter_tokens);

        let mut flat_builder = FlatNetworkFiltersListBuilder::new();
        let mut filter_map = HashMap::<Hash, Vec<u32>>::new();

        let mut optimizable = HashMap::<Hash, Vec<NetworkFilter>>::new();
        {
            for (network_filter, multi_tokens) in filter_tokens {
                let index = if !optimize
                    || !optimizer::is_filter_optimizable_by_patterns(&network_filter)
                {
                    Some(flat_builder.add(&network_filter))
                } else {
                    None
                };

                for tokens in multi_tokens {
                    let mut best_token: Hash = 0;
                    let mut min_count = total_number_of_tokens + 1;
                    for token in tokens {
                        match tokens_histogram.get(&token) {
                            None => {
                                min_count = 0;
                                best_token = token
                            }
                            Some(&count) if count < min_count => {
                                min_count = count;
                                best_token = token
                            }
                            _ => {}
                        }
                    }
                    if let Some(index) = index {
                        insert_dup(&mut filter_map, best_token, index);
                    } else {
                        insert_dup(&mut optimizable, best_token, network_filter.clone());
                    }
                } // tokens
            }
        }

        if optimize {
            for (token, v) in optimizable {
                let optimized = optimizer::optimize(v);

                for filter in optimized {
                    let index = flat_builder.add(&filter);
                    insert_dup(&mut filter_map, token, index);
                }
            }
        } else {
            debug_assert!(
                optimizable.is_empty(),
                "Should be empty if optimization is off"
            );
        }

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

        if filters_list.filter_map_index().len() == 0 {
            return None;
        }

        let filter_map = self.get_filter_map();

        for token in request.get_tokens_for_match() {
            for (index, fb_filter) in filter_map.get(token) {
                let filter = FlatNetworkFilter::new(&fb_filter, index as u32, self);

                // if matched, also needs to be tagged with an active tag (or not tagged at all)
                if filter.matches(request, regex_manager)
                    && filter.tag().map_or(true, |t| active_tags.contains(t))
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

        if filters_list.filter_map_index().len() == 0 {
            return filters;
        }

        let filter_map = self.get_filter_map();

        for token in request.get_tokens_for_match() {
            for (index, fb_filter) in filter_map.get(token) {
                let filter = FlatNetworkFilter::new(&fb_filter, index as u32, self);

                // if matched, also needs to be tagged with an active tag (or not tagged at all)
                if filter.matches(request, regex_manager)
                    && filter.tag().map_or(true, |t| active_tags.contains(t))
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
    let entry = map.entry(k).or_insert_with(Vec::new);

    match entry.binary_search_by(|f| f.partial_cmp(&v).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(_pos) => (), // Can occur if the exact same rule is inserted twice. No reason to add anything.
        Err(slot) => entry.insert(slot, v),
    }
}

pub(crate) fn token_histogram<T>(
    filter_tokens: &[(T, Vec<Vec<Hash>>)],
) -> (u32, HashMap<Hash, u32>) {
    let mut tokens_histogram: HashMap<Hash, u32> = HashMap::new();
    let mut number_of_tokens = 0;
    for (_, tokens) in filter_tokens.iter() {
        for tg in tokens {
            for t in tg {
                *tokens_histogram.entry(*t).or_insert(0) += 1;
                number_of_tokens += 1;
            }
        }
    }

    for bad_token in ["http", "https", "www", "com"].iter() {
        tokens_histogram.insert(fast_hash(bad_token), number_of_tokens);
    }

    (number_of_tokens, tokens_histogram)
}
