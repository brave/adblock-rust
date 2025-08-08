//! Builder for creating flatbuffer with serialized engine.
//! Currently the work in progress, therefore only some fields of Engine
//! are serialized to flatbuffer.
//! The entry point is `FlatBufferBuilder::make_flatbuffer`.

use std::collections::{HashMap, HashSet};
use std::vec;

use flatbuffers::WIPOffset;

use crate::cosmetic_filter_cache::CosmeticFilterCacheBuilder;
use crate::filters::flat_filter_map::{FlatFilterSetBuilder, FlatMultiMapBuilder};
use crate::filters::network::{NetworkFilter, NetworkFilterMaskHelper};
use crate::filters::unsafe_tools::VerifiedFlatbufferMemory;
use crate::network_filter_list::token_histogram;
use crate::optimizer;
use crate::resources::PermissionMask;
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

/// Accumulates hostname-specific rules for a single domain before building HostnameSpecificRules
#[derive(Default)]
struct HostnameRuleAccumulator {
    hide: Vec<String>,
    unhide: Vec<String>,
    inject_script: Vec<String>,
    inject_script_permissions: Vec<u32>,
    uninject_script: Vec<String>,
    procedural_action: Vec<String>,
    procedural_action_exception: Vec<String>,
}

pub(crate) struct FlatBufferBuilder {
    lists: Vec<FilterListBuilder>,

    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u32>,
    index: u32,
    simple_class_rules: FlatFilterSetBuilder<String>,
    simple_id_rules: FlatFilterSetBuilder<String>,
    misc_generic_selectors: FlatFilterSetBuilder<String>,
    complex_class_rules: FlatMultiMapBuilder<String, String>,
    complex_id_rules: FlatMultiMapBuilder<String, String>,

    // Hostname-specific rules using FlatMultiMapBuilder, store only one item per domain
    hostname_rules: HashMap<Hash, HostnameRuleAccumulator>,
}

impl FlatBufferBuilder {
    pub fn new(list_count: usize) -> Self {
        Self {
            lists: vec![FilterListBuilder::default(); list_count],
            unique_domains_hashes: vec![],
            unique_domains_hashes_map: HashMap::new(),
            index: 0,
            simple_class_rules: Default::default(),
            simple_id_rules: Default::default(),
            misc_generic_selectors: Default::default(),
            complex_class_rules: Default::default(),
            complex_id_rules: Default::default(),

            hostname_rules: HashMap::new(),
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

    pub fn add_simple_class_rule(&mut self, class_rule: String) {
        self.simple_class_rules.insert(class_rule);
    }

    pub fn add_simple_id_rule(&mut self, id_rule: String) {
        self.simple_id_rules.insert(id_rule);
    }

    pub fn add_misc_generic_selector(&mut self, selector: String) {
        self.misc_generic_selectors.insert(selector);
    }

    pub fn add_complex_class_rule(&mut self, class: String, selector: String) {
        self.complex_class_rules.insert(class, selector);
    }

    pub fn add_complex_id_rule(&mut self, id: String, selector: String) {
        self.complex_id_rules.insert(id, selector);
    }

    pub fn add_hostname_hide(&mut self, hash: Hash, selector: String) {
        self.hostname_rules.entry(hash).or_default().hide.push(selector);
    }

    pub fn add_hostname_unhide(&mut self, hash: Hash, selector: String) {
        self.hostname_rules.entry(hash).or_default().unhide.push(selector);
    }

    pub fn add_hostname_inject_script(
        &mut self,
        hash: Hash,
        selector: String,
        permission: PermissionMask,
    ) {
        let rules = self.hostname_rules.entry(hash).or_default();
        rules.inject_script.push(selector);
        // Store as u32, converting the u8 permission mask to u32
        let permission_bits = permission.0 as u32;
        rules.inject_script_permissions.push(permission_bits);
    }

    pub fn add_hostname_uninject_script(&mut self, hash: Hash, selector: String) {
        self.hostname_rules.entry(hash).or_default().uninject_script.push(selector);
    }

    pub fn add_hostname_procedural_action(&mut self, hash: Hash, json_data: String) {
        self.hostname_rules.entry(hash).or_default().procedural_action.push(json_data);
    }

    pub fn add_hostname_procedural_action_exception(&mut self, hash: Hash, json_data: String) {
        self.hostname_rules.entry(hash).or_default().procedural_action_exception.push(json_data);
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

        let simple_class_rules = std::mem::take(&mut self.simple_class_rules).finish(&mut builder);
        let simple_id_rules = std::mem::take(&mut self.simple_id_rules).finish(&mut builder);
        let misc_generic_selectors =
            std::mem::take(&mut self.misc_generic_selectors).finish(&mut builder);

        let (complex_class_rules_index, complex_class_rules_values) =
            std::mem::take(&mut self.complex_class_rules).finish(&mut builder);
        let (complex_id_rules_index, complex_id_rules_values) =
            std::mem::take(&mut self.complex_id_rules).finish(&mut builder);

        // Build HostnameSpecificRules for each domain
        let mut hostname_rules_map: HashMap<Hash, WIPOffset<fb::HostnameSpecificRules>> = HashMap::new();

        for (hash, rules) in std::mem::take(&mut self.hostname_rules) {
            // Create vectors for each rule type
            let hide_vec = if !rules.hide.is_empty() {
                let hide_offsets: Vec<_> = rules.hide.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&hide_offsets))
            } else { None };

            let unhide_vec = if !rules.unhide.is_empty() {
                let unhide_offsets: Vec<_> = rules.unhide.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&unhide_offsets))
            } else { None };

            let inject_script_vec = if !rules.inject_script.is_empty() {
                let script_offsets: Vec<_> = rules.inject_script.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&script_offsets))
            } else { None };

            let inject_script_permissions_vec = if !rules.inject_script_permissions.is_empty() {
                Some(builder.create_vector(&rules.inject_script_permissions))
            } else { None };

            let uninject_script_vec = if !rules.uninject_script.is_empty() {
                let uninject_offsets: Vec<_> = rules.uninject_script.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&uninject_offsets))
            } else { None };

            let procedural_action_vec = if !rules.procedural_action.is_empty() {
                let action_offsets: Vec<_> = rules.procedural_action.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&action_offsets))
            } else { None };

            let procedural_action_exception_vec = if !rules.procedural_action_exception.is_empty() {
                let exception_offsets: Vec<_> = rules.procedural_action_exception.iter().map(|s| builder.create_string(s)).collect();
                Some(builder.create_vector(&exception_offsets))
            } else { None };

            // Create HostnameSpecificRules
            let hostname_specific_rules = fb::HostnameSpecificRules::create(&mut builder, &fb::HostnameSpecificRulesArgs {
                hide: hide_vec,
                unhide: unhide_vec,
                inject_script: inject_script_vec,
                inject_script_permissions: inject_script_permissions_vec,
                uninject_script: uninject_script_vec,
                procedural_action: procedural_action_vec,
                procedural_action_exception: procedural_action_exception_vec,
            });

            hostname_rules_map.insert(hash, hostname_specific_rules);
        }

        // Use FlatMultiMapBuilder to create the hostname index and values
        let hostname_multimap = FlatMultiMapBuilder::new_from_map(
            hostname_rules_map.into_iter().map(|(k, v)| (k, vec![v])).collect()
        );
        let (hostname_index, hostname_values) = hostname_multimap.finish(&mut builder);

        let cosmetic_filters = fb::CosmeticFilters::create(
            &mut builder,
            &fb::CosmeticFiltersArgs {
                simple_class_rules: Some(simple_class_rules),
                simple_id_rules: Some(simple_id_rules),
                misc_generic_selectors: Some(misc_generic_selectors),
                complex_class_rules_index: Some(complex_class_rules_index),
                complex_class_rules_values: Some(complex_class_rules_values),
                complex_id_rules_index: Some(complex_id_rules_index),
                complex_id_rules_values: Some(complex_id_rules_values),
                hostname_index: Some(hostname_index),
                hostname_values: Some(hostname_values),
            },
        );

        let root = fb::Engine::create(
            &mut builder,
            &fb::EngineArgs {
                network_rules: Some(network_rules),
                unique_domains_hashes: Some(unique_vec),
                cosmetic_filters: Some(cosmetic_filters),
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

        let (indexes, values) = FlatMultiMapBuilder::new_from_map(filter_map).finish(builder);

        fb::NetworkFilterList::create(
            builder,
            &fb::NetworkFilterListArgs {
                filter_map_index: Some(indexes),
                filter_map_values: Some(values),
            },
        )
    }

    pub fn make_flatbuffer(
        network_filters: Vec<NetworkFilter>,
        cosmetic_cache_builder: &mut CosmeticFilterCacheBuilder,
        optimize: bool,
    ) -> VerifiedFlatbufferMemory {
        type FilterId = NetworkFilterListId;
        let mut builder = FlatBufferBuilder::new(FilterId::Size as usize);

        for class_rule in cosmetic_cache_builder.simple_class_rules.drain() {
            builder.add_simple_class_rule(class_rule);
        }

        for id_rule in cosmetic_cache_builder.simple_id_rules.drain() {
            builder.add_simple_id_rule(id_rule);
        }

        for selector in cosmetic_cache_builder.misc_generic_selectors.drain() {
            builder.add_misc_generic_selector(selector);
        }

        for (class, selectors) in cosmetic_cache_builder.complex_class_rules.drain() {
            for selector in selectors {
                builder.add_complex_class_rule(class.clone(), selector);
            }
        }

        for (id, selectors) in cosmetic_cache_builder.complex_id_rules.drain() {
            for selector in selectors {
                builder.add_complex_id_rule(id.clone(), selector);
            }
        }

        // Extract hostname filters from HostnameRuleDb
        let hostname_rules = &mut cosmetic_cache_builder.specific_rules;

        for (hash, selectors) in hostname_rules.hide.0.drain() {
            for selector in selectors {
                builder.add_hostname_hide(hash, selector);
            }
        }

        for (hash, selectors) in hostname_rules.unhide.0.drain() {
            for selector in selectors {
                builder.add_hostname_unhide(hash, selector);
            }
        }

        for (hash, script_data) in hostname_rules.inject_script.0.drain() {
            for (script, permission) in script_data {
                builder.add_hostname_inject_script(hash, script, permission);
            }
        }

        for (hash, selectors) in hostname_rules.uninject_script.0.drain() {
            for selector in selectors {
                builder.add_hostname_uninject_script(hash, selector);
            }
        }

        for (hash, json_data) in hostname_rules.procedural_action.0.drain() {
            for json in json_data {
                builder.add_hostname_procedural_action(hash, json);
            }
        }

        for (hash, json_data) in hostname_rules.procedural_action_exception.0.drain() {
            for json in json_data {
                builder.add_hostname_procedural_action_exception(hash, json);
            }
        }

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
