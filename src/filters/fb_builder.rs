//! Builder for creating flatbuffer with serialized engine.
//! Currently the work in progress, therefore only some fields of Engine
//! are serialized to flatbuffer.
//! The entry point is `FlatBufferBuilder::make_flatbuffer`.

use std::collections::{HashMap, HashSet};
use std::vec;

use flatbuffers::{ForwardsUOffset, Vector, WIPOffset};

use crate::cosmetic_filter_cache::CosmeticFilterCacheBuilder;
use crate::filters::cosmetic::CosmeticFilter;
use crate::filters::flat_filter_map::{FlatMultiMapBuilder, FlatSerialize, MyFlatBufferBuilder};
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
struct NetworkFilterListBuilder {
    filters: Vec<NetworkFilter>,
    optimize: bool,
}

struct NetworkRulesBuilder {
    lists: Vec<NetworkFilterListBuilder>,
}

impl<'a> FlatSerialize<'a> for NetworkFilter {
    type Output = WIPOffset<fb::NetworkFilter<'a>>;

    fn serialize(
        &mut self,
        builder: &mut MyFlatBufferBuilder<'a>,
    ) -> WIPOffset<fb::NetworkFilter<'a>> {
        let opt_domains = self.opt_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| builder.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            builder.create_vector(&o)
        });

        let opt_not_domains = self.opt_not_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| builder.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            builder.create_vector(&o)
        });

        let modifier_option = self
            .modifier_option
            .as_ref()
            .map(|s| builder.create_string(s));

        let hostname = self.hostname.as_ref().map(|s| builder.create_string(s));

        let tag = self.tag.as_ref().map(|s| builder.create_string(s));

        let patterns = if self.filter.iter().len() > 0 {
            let offsets: Vec<WIPOffset<&str>> = self
                .filter
                .iter()
                .map(|s| builder.create_string(s))
                .collect();
            Some(builder.create_vector(&offsets))
        } else {
            None
        };

        let raw_line = self
            .raw_line
            .as_ref()
            .map(|v| builder.create_string(v.as_str()));

        let filter = fb::NetworkFilter::create(
            &mut builder.fb_builder,
            &fb::NetworkFilterArgs {
                mask: self.mask.bits(),
                patterns,
                modifier_option,
                opt_domains,
                opt_not_domains,
                hostname,
                tag,
                raw_line,
            },
        );

        filter
    }
}

impl NetworkFilterListBuilder {
    fn new(optimize: bool) -> Self {
        Self {
            filters: vec![],
            optimize,
        }
    }
}

impl<'a> FlatSerialize<'a> for NetworkFilterListBuilder {
    type Output = WIPOffset<fb::NetworkFilterList<'a>>;
    fn serialize(
        &mut self,
        builder: &mut MyFlatBufferBuilder<'a>,
    ) -> WIPOffset<fb::NetworkFilterList<'a>> {
        let mut filter_map = HashMap::<ShortHash, Vec<WIPOffset<fb::NetworkFilter<'a>>>>::new();

        let mut optimizable = HashMap::<ShortHash, Vec<NetworkFilter>>::new();

        // Compute tokens for all filters
        let filter_tokens: Vec<_> = std::mem::take(&mut self.filters)
            .into_iter()
            .map(|filter| {
                let tokens = filter.get_tokens();
                (filter, tokens)
            })
            .collect();

        // compute the tokens' frequency histogram
        let (total_number_of_tokens, tokens_histogram) = token_histogram(&filter_tokens);

        {
            for (mut network_filter, multi_tokens) in filter_tokens.into_iter() {
                let flat_filter = if !self.optimize
                    || !optimizer::is_filter_optimizable_by_patterns(&network_filter)
                {
                    Some(network_filter.serialize(builder))
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

        if self.optimize {
            // Sort the entries to ensure deterministic iteration order
            let mut optimizable_entries: Vec<_> = optimizable.drain().collect();
            optimizable_entries.sort_unstable_by_key(|(token, _)| *token);

            for (token, v) in optimizable_entries {
                let optimized = optimizer::optimize(v);

                for mut filter in optimized {
                    let flat_filter = filter.serialize(builder);
                    filter_map.entry(token).or_default().push(flat_filter);
                }
            }
        } else {
            debug_assert!(
                optimizable.is_empty(),
                "Should be empty if optimization is off"
            );
        }

        let (indexes, values) = FlatMultiMapBuilder::new_from_map(filter_map).finish(builder);

        fb::NetworkFilterList::create(
            &mut builder.fb_builder,
            &fb::NetworkFilterListArgs {
                filter_map_index: Some(indexes),
                filter_map_values: Some(values),
            },
        )
    }
}

impl NetworkRulesBuilder {
    pub fn from_rules(network_filters: Vec<NetworkFilter>, optimize: bool) -> Self {
        let mut lists = vec![];
        for list_id in 0..NetworkFilterListId::Size as usize {
            // Don't optimize removeparam, since it can fuse filters without respecting distinct
            let optimize = optimize && list_id != NetworkFilterListId::RemoveParam as usize;
            lists.push(NetworkFilterListBuilder::new(optimize));
        }
        let mut self_ = Self { lists };

        let mut badfilter_ids: HashSet<Hash> = HashSet::new();
        for filter in network_filters.into_iter() {
            if filter.is_badfilter() {
                badfilter_ids.insert(filter.get_id_without_badfilter());
            }

            let filter_id = filter.get_id();
            if badfilter_ids.contains(&filter_id) || filter.is_badfilter() {
                continue;
            }

            // Redirects are independent of blocking behavior.
            if filter.is_redirect() {
                self_.add_filter(filter.clone(), NetworkFilterListId::Redirects);
            }
            type FilterId = NetworkFilterListId;

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

            self_.add_filter(filter, list_id);
        }

        self_
    }
    fn add_filter(&mut self, network_filter: NetworkFilter, list_id: NetworkFilterListId) {
        self.lists[list_id as usize].filters.push(network_filter);
    }
}

impl<'a> FlatSerialize<'a> for NetworkRulesBuilder {
    type Output = WIPOffset<Vector<'a, ForwardsUOffset<fb::NetworkFilterList<'a>>>>;
    fn serialize(&mut self, builder: &mut MyFlatBufferBuilder<'a>) -> Self::Output {
        let mut flat_network_rules = vec![];

        let lists = std::mem::take(&mut self.lists);
        for mut list in lists.into_iter() {
            flat_network_rules.push(list.serialize(builder));
        }
        builder.create_vector(&flat_network_rules)
    }
}

pub fn make_flatbuffer_from_rules(
    network_filters: Vec<NetworkFilter>,
    cosmetic_rules: Vec<CosmeticFilter>,
    optimize: bool,
) -> VerifiedFlatbufferMemory {
    let mut builder = MyFlatBufferBuilder::default();

    let mut network_builder = NetworkRulesBuilder::from_rules(network_filters, optimize);
    let flat_network_filters = network_builder.serialize(&mut builder);

    let mut cosmetic_builder = CosmeticFilterCacheBuilder::from_rules(cosmetic_rules);
    let flat_cosmetic_filters = cosmetic_builder.serialize(&mut builder);

    let flat_unique_domains_hashes = builder.write_unique_domains();

    let root = fb::Engine::create(
        &mut builder.fb_builder,
        &fb::EngineArgs {
            version: 1, // TODO
            network_rules: Some(flat_network_filters),
            unique_domains_hashes: Some(flat_unique_domains_hashes),
            cosmetic_filters: Some(flat_cosmetic_filters),
        },
    );
    builder.fb_builder.finish(root, None);
    // TODO: consider using builder.collapse() to avoid reallocating memory.
    VerifiedFlatbufferMemory::from_builder(&builder.fb_builder)
}
