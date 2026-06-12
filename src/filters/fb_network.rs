//! Flatbuffer-compatible versions of [NetworkFilter] and related functionality.

use crate::filters::filter_data_context::FilterDataContext;
use crate::filters::network::{NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable};
use crate::flatbuffers::unsafe_tools::fb_vector_to_slice;

use crate::regex_manager::RegexManager;
use crate::request::Request;

use crate::filters::flatbuffer_generated::fb;

/// A list of string parts that can be matched against a URL.
pub(crate) enum FlatPatterns<'a> {
    /// No patterns to match
    Empty,
    /// Memory-usage optimization - ~95% of filters have <= 1 pattern. Special-casing avoids the
    /// need to hold an extra pointer and vector length.
    Single(&'a str),
    /// More than 1 pattern to match
    Multi(flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>),
}

impl<'a> FlatPatterns<'a> {
    #[inline(always)]
    pub fn new(
        single_pattern: Option<&'a str>,
        multi_patterns: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
    ) -> Self {
        if let Some(single_pattern) = single_pattern {
            FlatPatterns::Single(single_pattern)
        } else if let Some(patterns) = multi_patterns {
            FlatPatterns::Multi(patterns)
        } else {
            FlatPatterns::Empty
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> FlatPatternsIterator<'_> {
        FlatPatternsIterator {
            patterns: self,
            index: 0,
        }
    }
}

/// Iterator over [FlatPatterns].
pub(crate) struct FlatPatternsIterator<'a> {
    patterns: &'a FlatPatterns<'a>,
    index: usize,
}

impl<'a> Iterator for FlatPatternsIterator<'a> {
    type Item = &'a str;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        match &self.patterns {
            FlatPatterns::Empty => None,
            FlatPatterns::Single(s) => {
                if self.index == 0 {
                    self.index += 1;
                    Some(*s)
                } else {
                    None
                }
            }
            FlatPatterns::Multi(v) => {
                if self.index < v.len() {
                    let result = v.get(self.index);
                    self.index += 1;
                    Some(result)
                } else {
                    None
                }
            }
        }
    }
}

impl ExactSizeIterator for FlatPatternsIterator<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        match &self.patterns {
            FlatPatterns::Empty => 0,
            FlatPatterns::Single(_) => 1_usize.saturating_sub(self.index),
            FlatPatterns::Multi(v) => v.len().saturating_sub(self.index),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ToOptionsFlags(u8);

impl ToOptionsFlags {
    const PLAIN_INC: u8 = 1 << 0;
    const PLAIN_EXC: u8 = 1 << 1;
    const ENTITY_INC: u8 = 1 << 2;
    const ENTITY_EXC: u8 = 1 << 3;

    #[inline(always)]
    pub fn any(self) -> bool {
        self.0 != 0
    }

    #[inline(always)]
    fn has_plain_inc(self) -> bool {
        self.0 & Self::PLAIN_INC != 0
    }

    #[inline(always)]
    fn has_plain_exc(self) -> bool {
        self.0 & Self::PLAIN_EXC != 0
    }

    #[inline(always)]
    fn has_entity_inc(self) -> bool {
        self.0 & Self::ENTITY_INC != 0
    }

    #[inline(always)]
    fn has_entity_exc(self) -> bool {
        self.0 & Self::ENTITY_EXC != 0
    }

    #[inline(always)]
    pub(crate) fn needs_plain(self) -> bool {
        self.0 & (Self::PLAIN_INC | Self::PLAIN_EXC) != 0
    }

    #[inline(always)]
    pub(crate) fn needs_entity(self) -> bool {
        self.0 & (Self::ENTITY_INC | Self::ENTITY_EXC) != 0
    }

    #[cfg(test)]
    pub(crate) fn from_bucket_parts(
        include_plain: bool,
        exclude_plain: bool,
        include_entity: bool,
        exclude_entity: bool,
    ) -> Self {
        let mut flags = 0u8;
        if include_plain {
            flags |= Self::PLAIN_INC;
        }
        if exclude_plain {
            flags |= Self::PLAIN_EXC;
        }
        if include_entity {
            flags |= Self::ENTITY_INC;
        }
        if exclude_entity {
            flags |= Self::ENTITY_EXC;
        }
        Self(flags)
    }

    fn from_filter(filter: &fb::NetworkFilter<'_>) -> Self {
        let mut flags = 0u8;
        if filter
            .opt_to_domains()
            .is_some_and(|data| !fb_vector_to_slice(data).is_empty())
        {
            flags |= Self::PLAIN_INC;
        }
        if filter
            .opt_not_to_domains()
            .is_some_and(|data| !fb_vector_to_slice(data).is_empty())
        {
            flags |= Self::PLAIN_EXC;
        }
        if filter
            .opt_to_entities()
            .is_some_and(|data| !fb_vector_to_slice(data).is_empty())
        {
            flags |= Self::ENTITY_INC;
        }
        if filter
            .opt_not_to_entities()
            .is_some_and(|data| !fb_vector_to_slice(data).is_empty())
        {
            flags |= Self::ENTITY_EXC;
        }
        Self(flags)
    }
}

/// Internal implementation of [NetworkFilter] that is compatible with flatbuffers.
pub(crate) struct FlatNetworkFilter<'a> {
    key: u64,
    filter_data_context: &'a FilterDataContext,
    fb_filter: &'a fb::NetworkFilter<'a>,

    pub(crate) mask: NetworkFilterMask,
    to_options: ToOptionsFlags,
}

impl<'a> FlatNetworkFilter<'a> {
    #[inline(always)]
    pub fn new(
        filter: &'a fb::NetworkFilter<'a>,
        filter_data_context: &'a FilterDataContext,
    ) -> Self {
        // Use the flatbuffer relative location as key, it's unique for
        // each filter regardless of the filter list it belongs to.
        let key = filter._tab.loc() as u64;

        Self {
            key,
            fb_filter: filter,
            mask: NetworkFilterMask::from_bits_retain(filter.mask()),
            to_options: ToOptionsFlags::from_filter(filter),
            filter_data_context,
        }
    }

    #[inline(always)]
    pub fn tag(&self) -> Option<&'a str> {
        self.fb_filter.tag()
    }

    #[inline(always)]
    pub fn modifier_option(&self) -> Option<String> {
        self.fb_filter.modifier_option().map(|o| o.to_string())
    }

    #[inline(always)]
    pub fn include_domains(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_domains()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn exclude_domains(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_not_domains()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn include_to_domains(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_to_domains()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn exclude_to_domains(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_not_to_domains()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn include_to_entities(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_to_entities()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn exclude_to_entities(&self) -> Option<&[u32]> {
        self.fb_filter
            .opt_not_to_entities()
            .map(|data| fb_vector_to_slice(data))
    }

    #[inline(always)]
    pub fn hostname(&self) -> Option<&'a str> {
        if self.mask.is_hostname_anchor() {
            self.fb_filter.hostname()
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn patterns(&self) -> FlatPatterns<'_> {
        FlatPatterns::new(
            self.fb_filter.single_pattern(),
            self.fb_filter.multi_patterns(),
        )
    }

    #[inline(always)]
    pub fn raw_line(&self) -> Option<String> {
        self.fb_filter.raw_line().map(|v| v.to_string())
    }
}

impl NetworkFilterMaskHelper for FlatNetworkFilter<'_> {
    #[inline]
    fn has_flag(&self, v: NetworkFilterMask) -> bool {
        self.mask.contains(v)
    }
}

impl NetworkMatchable for FlatNetworkFilter<'_> {
    fn matches(&self, request: &Request, regex_manager: &mut RegexManager) -> bool {
        use crate::filters::network_matchers::{
            check_excluded_domains_mapped, check_excluded_to_options_mapped,
            check_included_domains_mapped, check_included_to_options_mapped, check_options,
            check_pattern,
        };
        if !check_options(self.mask, request) {
            return false;
        }
        if !check_included_domains_mapped(
            self.include_domains(),
            request,
            &self.filter_data_context.unique_domains_hashes_map,
        ) {
            return false;
        }
        if !check_excluded_domains_mapped(
            self.exclude_domains(),
            request,
            &self.filter_data_context.unique_domains_hashes_map,
        ) {
            return false;
        }
        if self.to_options.any() {
            let mapping = &self.filter_data_context.unique_domains_hashes_map;
            let to_capability = self.filter_data_context.to_rule_capability();
            if !check_included_to_options_mapped(
                self.to_options,
                if self.to_options.has_plain_inc() {
                    self.include_to_domains()
                } else {
                    None
                },
                if self.to_options.has_entity_inc() {
                    self.include_to_entities()
                } else {
                    None
                },
                request,
                mapping,
                to_capability,
            ) {
                return false;
            }
            if !check_excluded_to_options_mapped(
                self.to_options,
                if self.to_options.has_plain_exc() {
                    self.exclude_to_domains()
                } else {
                    None
                },
                if self.to_options.has_entity_exc() {
                    self.exclude_to_entities()
                } else {
                    None
                },
                request,
                mapping,
                to_capability,
            ) {
                return false;
            }
        }
        check_pattern(
            self.mask,
            self.patterns().iter(),
            self.hostname(),
            self.key,
            request,
            regex_manager,
        )
    }
}
