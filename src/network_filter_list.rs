use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};
use crate::optimizer;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::{fast_hash, Hash};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

pub struct CheckResult {
    pub filter_mask: NetworkFilterMask,
    pub modifier_option: Option<String>,
}

impl NetworkFilterMaskHelper for CheckResult {
    #[inline]
    fn has_flag(&self, v: NetworkFilterMask) -> bool {
        self.filter_mask.contains(v)
    }
}

pub trait NetworkFilterListTrait {
    fn new(filters: Vec<NetworkFilter>, optimize: bool) -> Self
    where
        Self: Sized;
    fn optimize(&mut self);
    fn add_filter(&mut self, filter: NetworkFilter);
    fn filter_exists(&self, filter: &NetworkFilter) -> bool;

    fn check(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Option<CheckResult>;
    fn check_all(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Vec<CheckResult>;
}

#[derive(Serialize, Deserialize, Default)]
pub struct NetworkFilterList {
    #[serde(serialize_with = "crate::data_format::utils::stabilize_hashmap_serialization")]
    pub(crate) filter_map: HashMap<Hash, Vec<Arc<NetworkFilter>>>,
}

impl NetworkFilterListTrait for NetworkFilterList {
    fn new(filters: Vec<NetworkFilter>, optimize: bool) -> NetworkFilterList {
        // Compute tokens for all filters
        let filter_tokens: Vec<_> = filters
            .into_iter()
            .map(|filter| {
                let tokens = filter.get_tokens();
                (Arc::new(filter), tokens)
            })
            .collect();
        // compute the tokens' frequency histogram
        let (total_number_of_tokens, tokens_histogram) = token_histogram(&filter_tokens);

        // Build a HashMap of tokens to Network Filters (held through Arc, Atomic Reference Counter)
        let mut filter_map = HashMap::with_capacity(filter_tokens.len());
        {
            for (filter_pointer, multi_tokens) in filter_tokens {
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
                    insert_dup(&mut filter_map, best_token, Arc::clone(&filter_pointer));
                }
            }
        }

        let mut self_ = NetworkFilterList { filter_map };

        if optimize {
            self_.optimize();
        } else {
            self_.filter_map.shrink_to_fit();
        }

        self_
    }

    fn optimize(&mut self) {
        let mut optimized_map = HashMap::with_capacity(self.filter_map.len());
        for (key, filters) in self.filter_map.drain() {
            let mut unoptimized: Vec<NetworkFilter> = Vec::with_capacity(filters.len());
            let mut unoptimizable: Vec<Arc<NetworkFilter>> = Vec::with_capacity(filters.len());
            for f in filters {
                match Arc::try_unwrap(f) {
                    Ok(f) => unoptimized.push(f),
                    Err(af) => unoptimizable.push(af),
                }
            }

            let mut optimized: Vec<_> = if unoptimized.len() > 1 {
                optimizer::optimize(unoptimized)
                    .into_iter()
                    .map(Arc::new)
                    .collect()
            } else {
                // nothing to optimize
                unoptimized.into_iter().map(Arc::new).collect()
            };

            optimized.append(&mut unoptimizable);
            optimized.shrink_to_fit();
            optimized_map.insert(key, optimized);
        }

        // won't mutate anymore, shrink to fit items
        optimized_map.shrink_to_fit();

        self.filter_map = optimized_map;
    }

    fn add_filter(&mut self, filter: NetworkFilter) {
        let filter_tokens = filter.get_tokens();
        let total_rules = vec_hashmap_len(&self.filter_map);
        let filter_pointer = Arc::new(filter);

        for tokens in filter_tokens {
            let mut best_token: Hash = 0;
            let mut min_count = total_rules + 1;
            for token in tokens {
                match self.filter_map.get(&token) {
                    None => {
                        min_count = 0;
                        best_token = token
                    }
                    Some(filters) if filters.len() < min_count => {
                        min_count = filters.len();
                        best_token = token
                    }
                    _ => {}
                }
            }

            insert_dup(
                &mut self.filter_map,
                best_token,
                Arc::clone(&filter_pointer),
            );
        }
    }

    /// This may not work if the list has been optimized.
    fn filter_exists(&self, filter: &NetworkFilter) -> bool {
        let tokens: Vec<_> = filter.get_tokens().into_iter().flatten().collect();
        tokens.into_iter().chain(std::iter::once(0)).any(|token| {
            self.filter_map.get(&token).map_or(false, |filters| {
                filters
                    .iter()
                    .any(|saved_filter| saved_filter.id == filter.id)
            })
        })
    }

    /// Returns the first found filter, if any, that matches the given request. The backing storage
    /// has a non-deterministic order, so this should be used for any category of filters where a
    /// match from each would be functionally equivalent. For example, if two different exception
    /// filters match a certain request, it doesn't matter _which_ one is matched - the request
    /// will be excepted either way.
    fn check(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Option<CheckResult> {
        if self.filter_map.is_empty() {
            return None;
        }

        for token in request.checkable_tokens_iter() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter in filter_bucket {
                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request, regex_manager)
                        && filter
                            .tag
                            .as_ref()
                            .map(|t| active_tags.contains(t))
                            .unwrap_or(true)
                    {
                        return Some(CheckResult {
                            filter_mask: filter.mask,
                            modifier_option: filter.modifier_option.clone(),
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
    fn check_all(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Vec<CheckResult> {
        let mut filters: Vec<CheckResult> = vec![];

        if self.filter_map.is_empty() {
            return filters;
        }

        for token in request.checkable_tokens_iter() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter in filter_bucket {
                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request, regex_manager)
                        && filter
                            .tag
                            .as_ref()
                            .map(|t| active_tags.contains(t))
                            .unwrap_or(true)
                    {
                        filters.push(CheckResult {
                            filter_mask: filter.mask,
                            modifier_option: filter.modifier_option.clone(),
                        });
                    }
                }
            }
        }
        filters
    }
}

use crate::filters::fb_network::flat::fb;
use crate::filters::fb_network::{FlatNetworkFilter, FlatNetworkFiltersListBuilder};

pub struct FlatNetworkFilterList {
    flatbuffer_memory: Vec<u8>,
    pub(crate) filter_map: FlatMultimap<Hash, u32>,
    pub(crate) domain_hashes_mapping: HashMap<Hash, u16>,
}

fn total_bytes_in_hashmap(map: &HashMap<u64, Vec<u32>>) -> usize {
    let mut total_bytes = 0;

    // Memory usage of the HashMap itself
    // Each bucket in the HashMap stores a hash (usize), a key (u64), and a value (Vec<u32> pointer).
    // On a 64-bit system:
    // - Hash: 8 bytes
    // - Key: 8 bytes
    // - Value pointer: 8 bytes
    // Total per bucket: 24 bytes
    let hashmap_bucket_size = 8 + 8 + 8; // 24 bytes per bucket
    total_bytes += map.capacity() * hashmap_bucket_size;

    // Iterate over the HashMap to calculate memory usage of keys and values
    for (key, value) in map {
        // Size of the key (u64)
        total_bytes += std::mem::size_of::<u64>();

        // Size of the Vec<u32> metadata (pointer + length + capacity)
        total_bytes += std::mem::size_of::<Vec<u32>>();

        // Size of the u32 elements in the Vec (based on capacity, not len)
        total_bytes += value.capacity() * std::mem::size_of::<u32>();
    }

    total_bytes
}

impl NetworkFilterListTrait for FlatNetworkFilterList {
    fn new(filters: Vec<NetworkFilter>, _optimize: bool) -> Self {
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

        // Build a HashMap of tokens to Network Filters (held through Arc, Atomic Reference Counter)
        let mut flat_builder = FlatNetworkFiltersListBuilder::new();
        let mut filter_map = FlatMultimap::new();
        let mut total_index = 0;
        {
            for (filter, multi_tokens) in filter_tokens.into_iter() {
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
                    let index = flat_builder.add(&filter);
                    total_index = index;
                    insert_dup_b(&mut filter_map, best_token, index);
                }
            }
        }

        let flatbuffer_memory = flat_builder.finish();
        let root = fb::root_as_network_filter_list(&flatbuffer_memory)
            .expect("Ok because it is created in the previous line");

        let mut domain_hashes_mapping: HashMap<Hash, u16> = HashMap::new();
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            domain_hashes_mapping.insert(hash, u16::try_from(index).expect("< u16 max"));
        }

        filter_map.shrink_to_fit();
        domain_hashes_mapping.shrink_to_fit();

        /*println!(
            "bytes {} fm {} fmc {} dh {} total {} cap {}, len {}",
            flatbuffer_memory.len(),
            filter_map_n.len(),
            filter_map_n.capacity(),
            domain_hashes_mapping.len(),
            total_index,
            cap,
            len,
        );
        println!("---> filter_map {}", total_bytes_in_hashmap(&filter_map_n));*/

        Self {
            flatbuffer_memory,
            filter_map,
            domain_hashes_mapping,
        }
    }

    fn optimize(&mut self) {}

    fn add_filter(&mut self, _filter: NetworkFilter) {}

    fn filter_exists(&self, _filter: &NetworkFilter) -> bool {
        false
    }

    /// Returns the first found filter, if any, that matches the given request. The backing storage
    /// has a non-deterministic order, so this should be used for any category of filters where a
    /// match from each would be functionally equivalent. For example, if two different exception
    /// filters match a certain request, it doesn't matter _which_ one is matched - the request
    /// will be excepted either way.
    fn check(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Option<CheckResult> {
        if self.filter_map.is_empty() {
            return None;
        }

        let filters_list =
            unsafe { fb::root_as_network_filter_list_unchecked(&self.flatbuffer_memory) };
        let network_filters = filters_list.network_filters();

        for token in request.checkable_tokens_iter() {
            //if let Some(filter_bucket) = self.filter_map.get_all(token) {
            for filter_index in self.filter_map.get_all(token) {
                let fb_filter = network_filters.get(*filter_index as usize);
                let filter = FlatNetworkFilter::create(&fb_filter, *filter_index, self);

                // if matched, also needs to be tagged with an active tag (or not tagged at all)
                if filter.matches(request, regex_manager)
                    && filter.tag().map_or(true, |t| active_tags.contains(t))
                {
                    return Some(CheckResult {
                        filter_mask: filter.mask,
                        modifier_option: filter.modifier_option(),
                    });
                }
            }
            //}
        }

        None
    }

    /// Returns _all_ filters that match the given request. This should be used for any category of
    /// filters where a match from each may carry unique information. For example, if two different
    /// `$csp` filters match a certain request, they may each carry a distinct CSP directive, and
    /// each directive should be combined for the final result.
    fn check_all(
        &self,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Vec<CheckResult> {
        let mut filters: Vec<CheckResult> = vec![];

        /*if self.filter_map.is_empty() {
            return filters;
        }

        let filters_list =
            unsafe { fb::root_as_network_filter_list_unchecked(&self.flatbuffer_memory) };
        let network_filters = filters_list.network_filters();

        for token in request.checkable_tokens_iter() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter_index in filter_bucket {
                    let fb_filter = network_filters.get(*filter_index as usize);
                    let filter = FlatNetworkFilter::create(&fb_filter, *filter_index, self);

                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request, regex_manager)
                        && filter.tag().map_or(true, |t| active_tags.contains(t))
                    {
                        filters.push(CheckResult {
                            filter_mask: filter.mask,
                            modifier_option: filter.modifier_option(),
                        });
                    }
                }
            }
        }*/
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

pub(crate) fn insert_dup_b<K, V>(map: &mut FlatMultimap<K, V>, k: K, v: V)
where
    K: std::cmp::Ord + std::hash::Hash,
    V: PartialOrd,
{
    map.insert(k, v);
}

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

#[cfg(test)]
#[path = "../tests/unit/network_filter_list.rs"]
mod unit_tests;

#[derive(Debug)]
pub struct FlatMultimap<K, V> {
    data: Vec<(K, V)>,
}

impl<K, V> FlatMultimap<K, V>
where
    K: Ord,
{
    pub fn new() -> Self {
        FlatMultimap { data: Vec::new() }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let index = self.data.partition_point(|(k, _)| k < &key);
        self.data.insert(index, (key, value));
    }

    pub fn get_all<'a>(&'a self, key: &'a K) -> impl Iterator<Item = &V> + 'a {
        let start = self.find_first_index(key);
        self.data[start..]
            .iter()
            .take_while(move |(k, _)| k == key)
            .map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
        self.data.iter()
    }

    fn find_first_index(&self, key: &K) -> usize {
        self.data.partition_point(|(k, _)| k < key)
    }

    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }
}
