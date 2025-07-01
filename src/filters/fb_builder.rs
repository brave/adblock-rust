//! Builder for creating flatbuffer with serialized engine.
//! Currently the work in progress, therefore only some fields of Engine
//! are serialized to flatbuffer.
//! The entry point is `FlatBufferBuilder::make_flatbuffer`.

use std::collections::{HashMap, HashSet};
use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::{NetworkFilter, NetworkFilterMaskHelper};
use crate::filters::unsafe_tools::VerifiedFlatbufferMemory;
use crate::network_filter_list::token_histogram;
use crate::optimizer;
use crate::utils::{to_short_hash, Hash, ShortHash};

use super::fb_network::flat::fb;

pub(crate) enum NetworkFilterListId {
    Csp = 0,
    Exceptions = 1,
    Importants = 2,
    Redirects = 3,
    RemoveParam = 4,
    Filters = 5,
    GenericHide = 6,
    TaggedFiltersAll = 7,
    Size = 8,
}

#[derive(Default, Clone)]
struct FilterListBuilder {
    filters: Vec<NetworkFilter>,
}

pub(crate) struct FlatBufferBuilder {
    lists: Vec<FilterListBuilder>,

    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u32>,
    index: u32,
}

impl FlatBufferBuilder {
    pub fn new(list_count: usize) -> Self {
        Self {
            lists: vec![FilterListBuilder::default(); list_count],
            unique_domains_hashes: vec![],
            unique_domains_hashes_map: HashMap::new(),
            index: 0,
        }
    }

    fn get_or_insert_unique_domain_hash(&mut self, h: &Hash) -> u32 {
        if let Some(&index) = self.unique_domains_hashes_map.get(h) {
            return index;
        }
        let index = self.unique_domains_hashes.len() as u32;
        self.unique_domains_hashes.push(*h);
        self.unique_domains_hashes_map.insert(*h, index);
        index
    }

    pub fn add_filter(&mut self, network_filter: NetworkFilter, list_id: u32) {
        self.lists[list_id as usize].filters.push(network_filter);
    }

    fn write_filter<'a>(
        &mut self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
        network_filter: &NetworkFilter,
    ) -> WIPOffset<fb::NetworkFilter<'a>> {
        let opt_domains = network_filter.opt_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| self.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            builder.create_vector(&o)
        });

        let opt_not_domains = network_filter.opt_not_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| self.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            builder.create_vector(&o)
        });

        let modifier_option = network_filter
            .modifier_option
            .as_ref()
            .map(|s| builder.create_string(s));

        let hostname = network_filter
            .hostname
            .as_ref()
            .map(|s| builder.create_string(s));

        let tag = network_filter
            .tag
            .as_ref()
            .map(|s| builder.create_string(s));

        let patterns = if network_filter.filter.iter().len() > 0 {
            let offsets: Vec<WIPOffset<&str>> = network_filter
                .filter
                .iter()
                .map(|s| builder.create_string(s))
                .collect();
            Some(builder.create_vector(&offsets))
        } else {
            None
        };

        let raw_line = network_filter
            .raw_line
            .as_ref()
            .map(|v| builder.create_string(v.as_str()));

        let filter = fb::NetworkFilter::create(
            builder,
            &fb::NetworkFilterArgs {
                mask: network_filter.mask.bits(),
                patterns,
                modifier_option,
                opt_domains,
                opt_not_domains,
                hostname,
                tag,
                raw_line,
            },
        );

        self.index += 1;

        filter
    }

    pub fn finish(&mut self, optimize: bool) -> VerifiedFlatbufferMemory {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut flat_network_rules = vec![];

        let lists = std::mem::take(&mut self.lists);
        for (list_id, list) in lists.into_iter().enumerate() {
            // Don't optimize removeparam, since it can fuse filters without respecting distinct
            let optimize = optimize && list_id != NetworkFilterListId::RemoveParam as usize;

            flat_network_rules.push(self.write_filter_list(&mut builder, list.filters, optimize));
        }

        // Create vectors first to avoid simultaneous mutable borrows of `builder`.
        let network_rules = builder.create_vector(&flat_network_rules);
        let unique_vec = builder.create_vector(&self.unique_domains_hashes);

        let root = fb::Engine::create(
            &mut builder,
            &fb::EngineArgs {
                network_rules: Some(network_rules),
                unique_domains_hashes: Some(unique_vec),
            },
        );

        builder.finish(root, None);

        // TODO: consider using builder.collapse() to avoid reallocating memory.
        VerifiedFlatbufferMemory::from_builder(&builder)
    }

    pub fn write_filter_list<'a>(
        &mut self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
        filters: Vec<NetworkFilter>,
        optimize: bool,
    ) -> WIPOffset<fb::NetworkFilterList<'a>> {
        let mut filter_map = HashMap::<ShortHash, Vec<WIPOffset<fb::NetworkFilter<'a>>>>::new();

        let mut optimizable = HashMap::<ShortHash, Vec<NetworkFilter>>::new();

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

        {
            for (network_filter, multi_tokens) in filter_tokens {
                let flat_filter = if !optimize
                    || !optimizer::is_filter_optimizable_by_patterns(&network_filter)
                {
                    Some(self.write_filter(builder, &network_filter))
                } else {
                    None
                };

                for tokens in multi_tokens {
                    let mut best_token: ShortHash = 0;
                    let mut min_count = total_number_of_tokens + 1;
                    for token in tokens {
                        let token = to_short_hash(token);
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

                    if let Some(flat_filter) = flat_filter {
                        filter_map.entry(best_token).or_default().push(flat_filter);
                    } else {
                        optimizable
                            .entry(best_token)
                            .or_default()
                            .push(network_filter.clone());
                    }
                } // tokens
            }
        }

        if optimize {
            // Sort the entries to ensure deterministic iteration order
            let mut optimizable_entries: Vec<_> = optimizable.drain().collect();
            optimizable_entries.sort_unstable_by_key(|(token, _)| *token);

            for (token, v) in optimizable_entries {
                let optimized = optimizer::optimize(v);

                for filter in optimized {
                    let flat_filter = self.write_filter(builder, &filter);
                    filter_map.entry(token).or_default().push(flat_filter);
                }
            }
        } else {
            debug_assert!(
                optimizable.is_empty(),
                "Should be empty if optimization is off"
            );
        }

        let len = filter_map.len();

        // Convert filter_map keys to a sorted vector of (hash, filter_indices).
        let mut entries: Vec<_> = filter_map.drain().collect();
        entries.sort_unstable_by_key(|(k, _)| *k);

        // Convert sorted_entries to two flatbuffers vectors.
        let mut flat_index: Vec<ShortHash> = Vec::with_capacity(len);
        let mut flat_values: Vec<_> = Vec::with_capacity(len);
        for (key, filter_indices) in entries {
            for &filter_index in &filter_indices {
                flat_index.push(key);
                flat_values.push(filter_index);
            }
        }

        let filter_map_index = builder.create_vector(&flat_index);
        let filter_map_values = builder.create_vector(&flat_values);

        fb::NetworkFilterList::create(
            builder,
            &fb::NetworkFilterListArgs {
                filter_map_index: Some(filter_map_index),
                filter_map_values: Some(filter_map_values),
            },
        )
    }

    pub fn make_flatbuffer(
        network_filters: Vec<NetworkFilter>,
        optimize: bool,
    ) -> VerifiedFlatbufferMemory {
        type FilterId = NetworkFilterListId;
        let mut builder = FlatBufferBuilder::new(FilterId::Size as usize);

        let mut badfilter_ids: HashSet<Hash> = HashSet::new();
        for filter in network_filters.iter() {
            if filter.is_badfilter() {
                badfilter_ids.insert(filter.get_id_without_badfilter());
            }
        }
        for filter in network_filters.into_iter() {
            // skip any bad filters
            let filter_id = filter.get_id();
            if badfilter_ids.contains(&filter_id) || filter.is_badfilter() {
                continue;
            }

            // Redirects are independent of blocking behavior.
            if filter.is_redirect() {
                builder.add_filter(filter.clone(), FilterId::Redirects as u32);
            }

            let list_id: FilterId = if filter.is_csp() {
                FilterId::Csp
            } else if filter.is_removeparam() {
                FilterId::RemoveParam
            } else if filter.is_generic_hide() {
                FilterId::GenericHide
            } else if filter.is_exception() {
                FilterId::Exceptions
            } else if filter.is_important() {
                FilterId::Importants
            } else if filter.tag.is_some() && !filter.is_redirect() {
                // `tag` + `redirect` is unsupported for now.
                FilterId::TaggedFiltersAll
            } else if (filter.is_redirect() && filter.also_block_redirect())
                || !filter.is_redirect()
            {
                FilterId::Filters
            } else {
                continue;
            };

            builder.add_filter(filter, list_id as u32);
        }

        builder.finish(optimize)
    }
}
