use std::{collections::HashMap, collections::HashSet};

use crate::filters::fb_network::flat::fb;
use crate::filters::fb_network::{FlatNetworkFilter, FlatNetworkFiltersListBuilder};

use crate::filters::network::{NetworkFilter, NetworkMatchable};
use crate::network_filter_list::{insert_dup, token_histogram, NetworkFilterListTrait};

use crate::network_filter_list::CheckResult;
use crate::optimizer;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::Hash;

pub struct FlatNetworkFilterList {
    flatbuffer_memory: Vec<u8>,
    pub(crate) filter_map: HashMap<Hash, Vec<u32>>,
    pub(crate) domain_hashes_mapping: HashMap<Hash, u16>,
}

impl NetworkFilterListTrait for FlatNetworkFilterList {
    fn new(filters: Vec<NetworkFilter>, optimize: bool) -> Self {
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

        let flatbuffer_memory = flat_builder.finish();
        let root = fb::root_as_network_filter_list(&flatbuffer_memory)
            .expect("Ok because it is created in the previous line");

        let mut domain_hashes_mapping: HashMap<Hash, u16> = HashMap::new();
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            domain_hashes_mapping.insert(hash, u16::try_from(index).expect("< u16 max"));
        }

        filter_map.shrink_to_fit();
        domain_hashes_mapping.shrink_to_fit();

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

        for token in request.get_tokens_for_match() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter_index in filter_bucket {
                    let fb_filter = network_filters.get(*filter_index as usize);
                    let filter = FlatNetworkFilter::new(&fb_filter, *filter_index, self);

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

        let filters_list =
            unsafe { fb::root_as_network_filter_list_unchecked(&self.flatbuffer_memory) };
        let network_filters = filters_list.network_filters();

        for token in request.get_tokens_for_match() {
            if let Some(filter_bucket) = self.filter_map.get(token) {
                for filter_index in filter_bucket {
                    let fb_filter = network_filters.get(*filter_index as usize);
                    let filter = FlatNetworkFilter::new(&fb_filter, *filter_index, self);

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
