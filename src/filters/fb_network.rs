use crate::filters::network::{NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable};

use crate::network_filter_list::NetworkFilterList;
use crate::regex_manager::RegexManager;
use crate::request::Request;

// Cap'n Proto imports
use crate::network_filter_list::network_filter_capnp::network_filter;

// Pattern wrapper to match FlatBuffers interface
pub(crate) struct CapnpPatterns<'a> {
    patterns: Option<capnp::text_list::Reader<'a>>,
}

impl<'a> CapnpPatterns<'a> {
    #[inline(always)]
    pub fn new(patterns: Option<capnp::text_list::Reader<'a>>) -> Self {
        Self { patterns }
    }

    #[inline(always)]
    pub fn iter(&self) -> CapnpPatternsIterator<'a> {
        CapnpPatternsIterator {
            patterns: self.patterns,
            len: self.patterns.map_or(0, |d| d.len()),
            index: 0,
        }
    }
}

pub(crate) struct CapnpPatternsIterator<'a> {
    patterns: Option<capnp::text_list::Reader<'a>>,
    len: u32,
    index: u32,
}

impl<'a> Iterator for CapnpPatternsIterator<'a> {
    type Item = &'a str;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.patterns.and_then(|patterns| {
            if self.index < self.len {
                let result = patterns
                    .get(self.index)
                    .ok()
                    .and_then(|reader| reader.to_str().ok());
                self.index += 1;
                result
            } else {
                None
            }
        })
    }
}

impl<'a> ExactSizeIterator for CapnpPatternsIterator<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len as usize
    }
}

// Compatibility wrapper for existing NetworkFilterList interface - matches FlatNetworkFilter
pub(crate) struct CapnpNetworkFilter<'a> {
    key: u64,
    owner: &'a NetworkFilterList,
    capnp_filter: network_filter::Reader<'a>,
    pub(crate) mask: NetworkFilterMask,
}

impl<'a> CapnpNetworkFilter<'a> {
    #[inline(always)]
    pub fn new(
        filter: network_filter::Reader<'a>,
        index: u32,
        owner: &'a NetworkFilterList,
    ) -> Self {
        let mask = NetworkFilterMask::from_bits_truncate(filter.get_mask());
        let list_address: *const NetworkFilterList = owner as *const NetworkFilterList;
        let key = index as u64 | (((list_address) as u64) << 32);

        Self {
            capnp_filter: filter,
            key,
            mask,
            owner,
        }
    }

    #[inline(always)]
    pub fn tag(&self) -> Option<&'a str> {
        self.capnp_filter
            .get_tag()
            .ok()
            .and_then(|reader| reader.to_str().ok())
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
    }

    #[inline(always)]
    pub fn modifier_option(&self) -> Option<String> {
        self.capnp_filter
            .get_modifier_option()
            .ok()
            .and_then(|reader| reader.to_str().ok())
            .map(|s| s.to_string())
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
    }

    #[inline(always)]
    pub fn include_domains(&self) -> Option<&[u16]> {
        // For now, return None as domain slice access requires collecting from Cap'n Proto
        // This could be optimized later if needed
        None
    }

    #[inline(always)]
    pub fn exclude_domains(&self) -> Option<&[u16]> {
        // For now, return None as domain slice access requires collecting from Cap'n Proto
        // This could be optimized later if needed
        None
    }

    #[inline(always)]
    pub fn hostname(&self) -> Option<&'a str> {
        if self.mask.is_hostname_anchor() {
            self.capnp_filter
                .get_hostname()
                .ok()
                .and_then(|reader| reader.to_str().ok())
                .and_then(|s| if s.is_empty() { None } else { Some(s) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn patterns(&self) -> CapnpPatterns<'a> {
        CapnpPatterns::new(self.capnp_filter.get_patterns().ok())
    }

    #[inline(always)]
    pub fn raw_line(&self) -> Option<String> {
        self.capnp_filter
            .get_raw_line()
            .ok()
            .and_then(|reader| reader.to_str().ok())
            .map(|s| s.to_string())
            .and_then(|s| if s.is_empty() { None } else { Some(s) })
    }
}

impl<'a> NetworkFilterMaskHelper for CapnpNetworkFilter<'a> {
    #[inline]
    fn has_flag(&self, v: NetworkFilterMask) -> bool {
        self.mask.contains(v)
    }
}

impl<'a> NetworkMatchable for CapnpNetworkFilter<'a> {
    fn matches(&self, request: &Request, regex_manager: &mut RegexManager) -> bool {
        use crate::filters::network_matchers::{
            check_excluded_domains_mapped, check_included_domains_mapped, check_options,
            check_pattern,
        };
        if !check_options(self.mask, request) {
            return false;
        }
        if !check_included_domains_mapped(
            self.include_domains(),
            request,
            &self.owner.unique_domains_hashes_map,
        ) {
            return false;
        }
        if !check_excluded_domains_mapped(
            self.exclude_domains(),
            request,
            &self.owner.unique_domains_hashes_map,
        ) {
            return false;
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

    #[cfg(test)]
    fn matches_test(&self, request: &Request) -> bool {
        self.matches(request, &mut RegexManager::default())
    }
}
