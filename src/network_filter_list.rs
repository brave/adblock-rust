use std::{collections::HashMap, collections::HashSet, fmt};

use serde::{Deserialize, Serialize};

use crate::filters::fb_network::CapnpNetworkFilter;
use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};
use crate::optimizer;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::{fast_hash, Hash};

use capnp::message::{Builder, HeapAllocator, ReaderOptions};
use capnp::serialize;

// Include the Cap'n Proto generated code
#[allow(dead_code, unused_imports, unsafe_code)]
#[path = "capnp_network/network_filter_capnp.rs"]
pub mod network_filter_capnp;

use network_filter_capnp::network_filter_list;

// Builder for Cap'n Proto network filter list
struct CapnpNetworkFiltersListBuilder {
    message: Builder<HeapAllocator>,
    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u16>,
    // Store only essential data needed for building - much more compact than full NetworkFilter
    filters: Vec<CompactFilter>,
}

// Compact representation of a filter for building - avoids cloning full NetworkFilter
struct CompactFilter {
    mask: NetworkFilterMask,
    opt_domain_indices: Vec<u16>,
    opt_not_domain_indices: Vec<u16>,
    patterns: Vec<String>,
    modifier_option: Option<String>,
    hostname: Option<String>,
    tag: Option<String>,
    raw_line: Option<String>,
}

impl CapnpNetworkFiltersListBuilder {
    fn new() -> Self {
        Self {
            message: Builder::new_default(),
            unique_domains_hashes: vec![],
            unique_domains_hashes_map: HashMap::new(),
            filters: vec![],
        }
    }

    fn get_or_insert_unique_domain_hash(&mut self, h: &Hash) -> u16 {
        if let Some(&index) = self.unique_domains_hashes_map.get(h) {
            index
        } else {
            let index = self.unique_domains_hashes.len() as u16;
            self.unique_domains_hashes.push(*h);
            self.unique_domains_hashes_map.insert(*h, index);
            index
        }
    }

    fn add(&mut self, network_filter: &NetworkFilter) -> u32 {
        let opt_domain_indices: Vec<u16> = if let Some(ref domains) = network_filter.opt_domains {
            domains
                .iter()
                .map(|h| self.get_or_insert_unique_domain_hash(h))
                .collect()
        } else {
            Vec::new()
        };

        let opt_not_domain_indices: Vec<u16> = if let Some(ref domains) = network_filter.opt_not_domains {
            domains
                .iter()
                .map(|h| self.get_or_insert_unique_domain_hash(h))
                .collect()
        } else {
            Vec::new()
        };

        let patterns: Vec<String> = network_filter.filter.iter().map(|s| s.to_string()).collect();

        self.filters.push(CompactFilter {
            mask: network_filter.mask,
            opt_domain_indices,
            opt_not_domain_indices,
            patterns,
            modifier_option: network_filter.modifier_option.clone(),
            hostname: network_filter.hostname.clone(),
            tag: network_filter.tag.clone(),
            raw_line: network_filter.raw_line.as_ref().map(|s| s.as_str().to_string()),
        });

        (self.filters.len() - 1) as u32
    }

    fn finish(mut self) -> Vec<u8> {
        let mut root = self.message.init_root::<network_filter_list::Builder>();

        // Set unique domains hashes
        let mut unique_domains_hashes = root.reborrow().init_unique_domains_hashes(self.unique_domains_hashes.len() as u32);
        for (index, hash) in self.unique_domains_hashes.iter().enumerate() {
            unique_domains_hashes.set(index as u32, *hash);
        }

        // Set network filters
        let mut network_filters = root.reborrow().init_network_filters(self.filters.len() as u32);
        for (i, filter_data) in self.filters.iter().enumerate() {
            let mut filter_builder = network_filters.reborrow().get(i as u32);

            // Set mask
            filter_builder.set_mask(filter_data.mask.bits());

            // Set opt domains
            if !filter_data.opt_domain_indices.is_empty() {
                let mut opt_domains = filter_builder.reborrow().init_opt_domains(filter_data.opt_domain_indices.len() as u32);
                for (j, &domain_index) in filter_data.opt_domain_indices.iter().enumerate() {
                    opt_domains.set(j as u32, domain_index);
                }
            }

            // Set opt not domains
            if !filter_data.opt_not_domain_indices.is_empty() {
                let mut opt_not_domains = filter_builder.reborrow().init_opt_not_domains(filter_data.opt_not_domain_indices.len() as u32);
                for (j, &domain_index) in filter_data.opt_not_domain_indices.iter().enumerate() {
                    opt_not_domains.set(j as u32, domain_index);
                }
            }

            // Set patterns
            if !filter_data.patterns.is_empty() {
                let mut patterns_builder = filter_builder.reborrow().init_patterns(filter_data.patterns.len() as u32);
                for (j, pattern) in filter_data.patterns.iter().enumerate() {
                    patterns_builder.set(j as u32, pattern);
                }
            }

            // Set optional strings
            if let Some(ref modifier) = filter_data.modifier_option {
                filter_builder.reborrow().set_modifier_option(modifier);
            }
            if let Some(ref hostname) = filter_data.hostname {
                filter_builder.reborrow().set_hostname(hostname);
            }
            if let Some(ref tag) = filter_data.tag {
                filter_builder.reborrow().set_tag(tag);
            }
            if let Some(ref raw_line) = filter_data.raw_line {
                filter_builder.reborrow().set_raw_line(raw_line);
            }
        }

        // Serialize the message to bytes - this consumes self.message
        let mut buffer = Vec::new();
        serialize::write_message(&mut buffer, &self.message).expect("Failed to serialize message");
        buffer
        // Builder and all internal data is dropped here automatically
    }
}

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

#[derive(Serialize, Deserialize)]
pub(crate) struct NetworkFilterList {
    pub(crate) flatbuffer_memory: Vec<u8>,
    pub(crate) filter_map: HashMap<Hash, Vec<u32>>,
    pub(crate) unique_domains_hashes_map: HashMap<Hash, u16>,
}

impl Default for NetworkFilterList {
    fn default() -> Self {
        Self {
            flatbuffer_memory: Default::default(),
            filter_map: Default::default(),
            unique_domains_hashes_map: Default::default(),
        }
    }
}

impl NetworkFilterList {
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

        let mut flat_builder = CapnpNetworkFiltersListBuilder::new();
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

        // Builder is consumed here and all internal data is dropped
        let flatbuffer_memory = flat_builder.finish();

        let mut slice = flatbuffer_memory.as_slice();
        let reader = serialize::read_message_from_flat_slice(&mut slice, ReaderOptions::new())
            .expect("Ok because it is created in the previous line");
        let root = reader.get_root::<network_filter_list::Reader>()
            .expect("Ok because it is created in the previous line");

        let mut unique_domains_hashes_map: HashMap<Hash, u16> = HashMap::new();
        if let Ok(unique_domains_hashes) = root.get_unique_domains_hashes() {
            for (index, hash) in unique_domains_hashes.iter().enumerate() {
                unique_domains_hashes_map.insert(hash, u16::try_from(index).expect("< u16 max"));
            }
        }

        filter_map.shrink_to_fit();
        unique_domains_hashes_map.shrink_to_fit();

        Self {
            flatbuffer_memory,
            filter_map,
            unique_domains_hashes_map,
        }
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
        if self.filter_map.is_empty() {
            return None;
        }

        let mut slice = self.flatbuffer_memory.as_slice();
        let reader = serialize::read_message_from_flat_slice(&mut slice, ReaderOptions::new()).ok()?;
        let filters_list = reader.get_root::<network_filter_list::Reader>().ok()?;
        let network_filters = filters_list.get_network_filters().ok()?;

        for token in request.get_tokens_for_match() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter_index in filter_bucket {
                    let fb_filter = network_filters.get(*filter_index);
                    let filter = CapnpNetworkFilter::new(fb_filter, *filter_index, self);

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

        if self.filter_map.is_empty() {
            return filters;
        }

        let mut slice = self.flatbuffer_memory.as_slice();
        let reader = match serialize::read_message_from_flat_slice(&mut slice, ReaderOptions::new()) {
            Ok(reader) => reader,
            Err(_) => return filters,
        };
        let filters_list = match reader.get_root::<network_filter_list::Reader>() {
            Ok(list) => list,
            Err(_) => return filters,
        };
        let network_filters = match filters_list.get_network_filters() {
            Ok(filters) => filters,
            Err(_) => return filters,
        };

        for token in request.get_tokens_for_match() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter_index in filter_bucket {
                    let fb_filter = network_filters.get(*filter_index);
                    let filter = CapnpNetworkFilter::new(fb_filter, *filter_index, self);

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

#[cfg(test)]
pub(crate) fn vec_hashmap_len<K: std::cmp::Eq + std::hash::Hash, V, H: std::hash::BuildHasher>(
    map: &HashMap<K, Vec<V>, H>,
) -> usize {
    let mut size = 0usize;
    for (_, val) in map.iter() {
        size += val.len();
    }
    size
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
