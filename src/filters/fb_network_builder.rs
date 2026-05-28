//! Structures to store network filters to flatbuffer

use std::collections::{HashMap, HashSet};

use flatbuffers::WIPOffset;

use crate::filters::fb_builder::EngineFlatBuilder;
use crate::filters::network::{FilterTokens, NetworkFilter};
use crate::filters::token_selector::TokenSelector;
use crate::utils::TokensBuffer;

use crate::filters::network::NetworkFilterMaskHelper;
use crate::flatbuffers::containers::flat_multimap::FlatMultiMapBuilder;
use crate::flatbuffers::containers::flat_serialize::{FlatBuilder, FlatSerialize};
use crate::optimizer;
use crate::utils::{to_short_hash, Hash, ShortHash};

use super::flat::fb;

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

struct NetworkFilterFlatEntry<'a> {
    filter: WIPOffset<fb::NetworkFilter<'a>>,
    id: Hash,
}

pub(crate) struct NetworkFilterListBuilder<'a> {
    flat_map_builder: FlatMultiMapBuilder<ShortHash, NetworkFilterFlatEntry<'a>>,
    token_frequencies: TokenSelector,
    filters_to_optimize: HashMap<ShortHash, Vec<NetworkFilter>>,
    tokens_buffer: TokensBuffer,
    optimize: bool,
}

pub(crate) struct NetworkRulesBuilder<'a> {
    lists: Vec<NetworkFilterListBuilder<'a>>,
    bad_filter_ids: HashSet<Hash>,
}

impl<'a> FlatSerialize<'a, EngineFlatBuilder<'a>> for NetworkFilter {
    type Output = WIPOffset<fb::NetworkFilter<'a>>;

    fn serialize(
        network_filter: NetworkFilter,
        builder: &mut EngineFlatBuilder<'a>,
    ) -> WIPOffset<fb::NetworkFilter<'a>> {
        let opt_domains = network_filter.opt_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| builder.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            FlatSerialize::serialize(o, builder)
        });

        let opt_not_domains = network_filter.opt_not_domains.as_ref().map(|v| {
            let mut o: Vec<u32> = v
                .iter()
                .map(|x| builder.get_or_insert_unique_domain_hash(x))
                .collect();
            o.sort_unstable();
            o.dedup();
            FlatSerialize::serialize(o, builder)
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

        let mut filter_iter = network_filter.filter.iter();
        let filter_count = filter_iter.len();

        // Use single_pattern for the common case of 0 or 1 patterns to avoid
        // the overhead of a FlatBuffers vector (extra table + offset indirection).
        let (single_pattern, multi_patterns) = if filter_count <= 1 {
            let single = filter_iter.next().map(|s| builder.create_string(s));
            (single, None)
        } else {
            (
                None,
                Some(FlatSerialize::serialize(
                    filter_iter.collect::<Vec<_>>(),
                    builder,
                )),
            )
        };

        let raw_line = network_filter
            .raw_line
            .as_ref()
            .map(|v| builder.create_string(v.as_str()));

        fb::NetworkFilter::create(
            builder.raw_builder(),
            &fb::NetworkFilterArgs {
                mask: network_filter.mask.bits(),
                single_pattern,
                multi_patterns,
                modifier_option,
                opt_domains,
                opt_not_domains,
                hostname,
                tag,
                raw_line,
            },
        )
    }
}

impl<'a> NetworkFilterListBuilder<'a> {
    fn new(optimize: bool) -> Self {
        Self {
            flat_map_builder: FlatMultiMapBuilder::with_capacity(1024),
            token_frequencies: TokenSelector::new(0),
            filters_to_optimize: HashMap::new(),
            tokens_buffer: TokensBuffer::default(),
            optimize,
        }
    }

    fn add_filter(&mut self, network_filter: NetworkFilter, builder: &mut EngineFlatBuilder<'a>) {
        let multi_tokens = network_filter.get_tokens(&mut self.tokens_buffer);
        let id = network_filter.get_id();

        // Resolve token(s) and record frequencies up-front so the
        // serialized/optimizable branches share no token logic.
        let mut single_token: Hash = 0;
        let tokens: &[Hash] = match multi_tokens {
            FilterTokens::Empty => std::slice::from_ref(&single_token),
            FilterTokens::OptDomains => {
                // tokens_buffer has been populated by get_tokens; record frequencies
                let slice = self.tokens_buffer.as_slice();
                for &t in slice {
                    self.token_frequencies.record_usage(t);
                }
                slice
            }
            FilterTokens::Other => {
                single_token = self
                    .token_frequencies
                    .select_least_used_token(self.tokens_buffer.as_slice());
                self.token_frequencies.record_usage(single_token);
                std::slice::from_ref(&single_token)
            }
        };

        if !self.optimize || !optimizer::is_filter_optimizable_by_patterns(&network_filter) {
            // Serialize immediately; store offset + id for later bad-filter pruning.
            let filter = FlatSerialize::serialize(network_filter, builder);
            for &token in tokens {
                self.flat_map_builder
                    .insert(to_short_hash(token), NetworkFilterFlatEntry { filter, id });
            }
        } else {
            // Defer to the optimizer; clone only in the multi-token (OptDomains) case.
            if let Some((last, rest)) = tokens.split_last() {
                for &token in rest {
                    self.filters_to_optimize
                        .entry(to_short_hash(token))
                        .or_default()
                        .push(network_filter.clone());
                }
                self.filters_to_optimize
                    .entry(to_short_hash(*last))
                    .or_default()
                    .push(network_filter);
            }
        }
    }
}

impl<'a> NetworkRulesBuilder<'a> {
    pub fn new(optimize: bool) -> Self {
        let mut lists = vec![];
        for list_id in 0..NetworkFilterListId::Size as usize {
            // Don't optimize removeparam, since it can fuse filters without respecting distinct
            let optimize = optimize && list_id != NetworkFilterListId::RemoveParam as usize;
            lists.push(NetworkFilterListBuilder::new(optimize));
        }
        Self {
            lists,
            bad_filter_ids: HashSet::new(),
        }
    }

    pub fn add_rules(&mut self, filter: NetworkFilter, builder: &mut EngineFlatBuilder<'a>) {
        if filter.is_badfilter() {
            // `get_id()` already excludes the BAD_FILTER bit, so this ID matches
            // the corresponding normal filter that this badfilter is meant to cancel.
            self.bad_filter_ids.insert(filter.get_id());
            return;
        }

        // Redirects are independent of blocking behavior.
        if filter.is_redirect() {
            self.add_filter(filter.clone(), NetworkFilterListId::Redirects, builder);
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
        } else if (filter.is_redirect() && filter.also_block_redirect()) || !filter.is_redirect() {
            FilterId::Filters
        } else {
            return;
        };

        self.add_filter(filter, list_id, builder);
    }

    fn add_filter(
        &mut self,
        network_filter: NetworkFilter,
        list_id: NetworkFilterListId,
        builder: &mut EngineFlatBuilder<'a>,
    ) {
        self.lists[list_id as usize].add_filter(network_filter, builder);
    }
}

impl<'a> FlatSerialize<'a, EngineFlatBuilder<'a>> for NetworkFilterFlatEntry<'a> {
    type Output = WIPOffset<fb::NetworkFilter<'a>>;

    fn serialize(
        value: Self,
        _builder: &mut EngineFlatBuilder<'a>,
    ) -> WIPOffset<fb::NetworkFilter<'a>> {
        value.filter
    }
}

impl<'a> FlatSerialize<'a, EngineFlatBuilder<'a>> for NetworkRulesBuilder<'a> {
    type Output =
        WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<fb::NetworkFilterList<'a>>>>;

    fn serialize(value: Self, builder: &mut EngineFlatBuilder<'a>) -> Self::Output {
        let mut serialized_lists = vec![];

        for mut rule_list in value.lists {
            if !rule_list.filters_to_optimize.is_empty() {
                // Sort entries for deterministic iteration order.
                let mut optimizable_entries: Vec<_> =
                    rule_list.filters_to_optimize.drain().collect();
                optimizable_entries.sort_unstable_by_key(|(token, _)| *token);

                for (token, mut v) in optimizable_entries {
                    v.retain(|f| !value.bad_filter_ids.contains(&f.get_id()));
                    let optimized = optimizer::optimize(v);

                    for filter in optimized {
                        let id = filter.get_id();
                        let filter = FlatSerialize::serialize(filter, builder);
                        rule_list
                            .flat_map_builder
                            .insert(token, NetworkFilterFlatEntry { filter, id });
                    }
                }
            }

            // Prune already-serialized entries that were cancelled by a $badfilter.
            rule_list
                .flat_map_builder
                .retain_by_value(|entry| !value.bad_filter_ids.contains(&entry.id));

            let flat_filter_map = FlatMultiMapBuilder::finish(rule_list.flat_map_builder, builder);

            serialized_lists.push(fb::NetworkFilterList::create(
                builder.raw_builder(),
                &fb::NetworkFilterListArgs {
                    filter_map_index: Some(flat_filter_map.keys),
                    filter_map_values: Some(flat_filter_map.values),
                },
            ));
        }

        FlatSerialize::serialize(serialized_lists, builder)
    }
}
