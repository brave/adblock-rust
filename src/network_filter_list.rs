//! Holds the implementation of [NetworkFilterList] and related functionality.

use std::{collections::HashSet, fmt};

use flatbuffers::ForwardsUOffset;

use crate::filters::fb_network::FlatNetworkFilter;
use crate::filters::filter_data_context::FilterDataContext;
use crate::filters::flatbuffer_generated::fb;
use crate::filters::network::{NetworkFilterMask, NetworkMatchable};
use crate::flatbuffers::containers::flat_multimap::FlatMultiMapView;
use crate::flatbuffers::unsafe_tools::fb_vector_to_slice;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::sourcemap::FilterRuleDebugInfo;
use crate::utils::{to_short_hash, Hash, ShortHash};

/// Holds relevant information from a single matching network filter rule as a result of querying a
/// [NetworkFilterList] for a given request.
pub(crate) struct CheckResult {
    pub filter_mask: NetworkFilterMask,
    pub modifier_option: Option<String>,
    pub debug_data: Option<FilterRuleDebugInfo>,
}

impl fmt::Display for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(ref debug_data) = self.debug_data {
            write!(f, "{debug_data}")
        } else {
            write!(f, "{}", self.filter_mask)
        }
    }
}

/// Internal structure to keep track of a collection of network filters.
pub(crate) struct NetworkFilterList<'a> {
    pub(crate) list: fb::NetworkFilterList<'a>,
    pub(crate) filter_data_context: &'a FilterDataContext,
}

type FlatNetworkFilterMap<'a> =
    FlatMultiMapView<'a, ShortHash, ForwardsUOffset<fb::NetworkFilter<'a>>, &'a [ShortHash]>;

impl NetworkFilterList<'_> {
    pub fn get_filter_map(&self) -> FlatNetworkFilterMap<'_> {
        FlatNetworkFilterMap::new(
            fb_vector_to_slice(self.list.filter_map_index()),
            self.list.filter_map_values(),
        )
    }

    pub fn get_opt_domains_map(&self) -> FlatNetworkFilterMap<'_> {
        FlatNetworkFilterMap::new(
            fb_vector_to_slice(self.list.opt_domains_map_index()),
            self.list.opt_domains_map_values(),
        )
    }

    pub fn get_opt_to_domains_map(&self) -> FlatNetworkFilterMap<'_> {
        FlatNetworkFilterMap::new(
            fb_vector_to_slice(self.list.opt_to_domains_map_index()),
            self.list.opt_to_domains_map_values(),
        )
    }

    fn is_empty(&self) -> bool {
        self.list.filter_map_index().is_empty()
            && self.list.opt_domains_map_index().is_empty()
            && self.list.opt_to_domains_map_index().is_empty()
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
        if self.is_empty() {
            return None;
        }

        if let Some(result) = self.check_tokens_in_map(
            &self.get_opt_domains_map(),
            request.get_tokens_for_match(),
            request,
            active_tags,
            regex_manager,
        ) {
            return Some(result);
        }

        if let Some(result) = self.check_tokens_in_map(
            &self.get_opt_to_domains_map(),
            request.get_tokens_for_to_match(),
            request,
            active_tags,
            regex_manager,
        ) {
            return Some(result);
        }

        self.check_tokens_in_map(
            &self.get_filter_map(),
            request
                .get_tokens_for_match()
                .chain(request.get_tokens_for_to_match())
                .chain(request.get_tokens().iter()),
            request,
            active_tags,
            regex_manager,
        )
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

        if self.is_empty() {
            return filters;
        }

        self.collect_tokens_in_map(
            &self.get_opt_domains_map(),
            request.get_tokens_for_match(),
            request,
            active_tags,
            regex_manager,
            &mut filters,
        );
        self.collect_tokens_in_map(
            &self.get_opt_to_domains_map(),
            request.get_tokens_for_to_match(),
            request,
            active_tags,
            regex_manager,
            &mut filters,
        );
        self.collect_tokens_in_map(
            &self.get_filter_map(),
            request
                .get_tokens_for_match()
                .chain(request.get_tokens_for_to_match())
                .chain(request.get_tokens().iter()),
            request,
            active_tags,
            regex_manager,
            &mut filters,
        );

        filters
    }

    fn check_tokens_in_map<'a, I>(
        &self,
        filter_map: &FlatNetworkFilterMap<'_>,
        tokens: I,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
    ) -> Option<CheckResult>
    where
        I: IntoIterator<Item = &'a Hash>,
    {
        for token in tokens {
            if let Some(iter) = filter_map.get(to_short_hash(*token)) {
                for fb_filter in iter {
                    let filter = FlatNetworkFilter::new(&fb_filter, self.filter_data_context);

                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request, regex_manager)
                        && filter.tag().is_none_or(|t| active_tags.contains(t))
                    {
                        return Some(CheckResult {
                            filter_mask: filter.mask,
                            modifier_option: filter.modifier_option(),
                            debug_data: filter.get_rule_debug_info(),
                        });
                    }
                }
            }
        }
        None
    }

    fn collect_tokens_in_map<'a, I>(
        &self,
        filter_map: &FlatNetworkFilterMap<'_>,
        tokens: I,
        request: &Request,
        active_tags: &HashSet<String>,
        regex_manager: &mut RegexManager,
        filters: &mut Vec<CheckResult>,
    ) where
        I: IntoIterator<Item = &'a Hash>,
    {
        for token in tokens {
            if let Some(iter) = filter_map.get(to_short_hash(*token)) {
                for fb_filter in iter {
                    let filter = FlatNetworkFilter::new(&fb_filter, self.filter_data_context);

                    // if matched, also needs to be tagged with an active tag (or not tagged at all)
                    if filter.matches(request, regex_manager)
                        && filter.tag().is_none_or(|t| active_tags.contains(t))
                    {
                        filters.push(CheckResult {
                            filter_mask: filter.mask,
                            modifier_option: filter.modifier_option(),
                            debug_data: filter.get_rule_debug_info(),
                        });
                    }
                }
            }
        }
    }
}
