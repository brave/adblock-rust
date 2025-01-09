use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::{NetworkFilter, NetworkFilterMask};
use crate::regex_manager::RegexManager;
use crate::request::{self};
use crate::utils::Hash;

extern crate flatbuffers;
#[allow(dead_code, unused_imports, unsafe_code)]
#[path = "../flat/fb_network_filter_generated.rs"]
pub mod flat;
use flat::fb;

use super::network::NetworkMatchable;

pub struct FlatNetworkFiltersListBuilder<'a> {
    builder: flatbuffers::FlatBufferBuilder<'a>,
    filters: Vec<WIPOffset<fb::NetworkFilter<'a>>>,
}

impl<'a> FlatNetworkFiltersListBuilder<'a> {
    pub fn new() -> Self {
        Self {
            builder: flatbuffers::FlatBufferBuilder::new(),
            filters: vec![],
        }
    }

    pub fn add(&mut self, network_filter: NetworkFilter) -> u32 {
        let opt_domains = network_filter
            .opt_domains
            .map(|v| self.builder.create_vector(&v));

        let opt_not_domains = network_filter
            .opt_not_domains
            .map(|v| self.builder.create_vector(&v));

        let modifier_option = network_filter
            .modifier_option
            .map(|s| self.builder.create_shared_string(&s));

        let hostname = network_filter
            .hostname
            .map(|s| self.builder.create_string(&s));

        let tag = network_filter
            .tag
            .map(|s| self.builder.create_shared_string(&s));

        let patterns = if network_filter.filter.iter().len() > 0 {
            let offsets: Vec<WIPOffset<&str>> = network_filter
                .filter
                .iter()
                .map(|s| self.builder.create_string(s))
                .collect();
            Some(self.builder.create_vector(&offsets))
        } else {
            None
        };

        let filter = fb::NetworkFilter::create(
            &mut self.builder,
            &fb::NetworkFilterArgs {
                mask: network_filter.mask.bits(),
                patterns: patterns,
                modifier_option: modifier_option,
                opt_domains: opt_domains,
                opt_not_domains: opt_not_domains,
                hostname: hostname,
                tag: tag,
            },
        );

        self.filters.push(filter);
        u32::try_from(self.filters.len() - 1).expect("< u32::MAX")
    }

    pub fn finish(&mut self) -> Vec<u8> {
        let filters = self.builder.create_vector(&self.filters);

        let storage = fb::NetworkFilterList::create(
            &mut self.builder,
            &&fb::NetworkFilterListArgs {
                global_list: Some(filters),
            },
        );
        self.builder.finish(storage, None);

        Vec::from(self.builder.finished_data())
    }
}
pub struct FlatPatterns<'a> {
    data: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
}

impl<'a> FlatPatterns<'a> {
    #[inline(always)]
    pub fn iter(&self) -> FlatPatternsIterator {
        FlatPatternsIterator {
            patterns: self,
            len: self.data.map_or(0, |d| d.len()),
            index: 0,
        }
    }
}

pub struct FlatPatternsIterator<'a> {
    patterns: &'a FlatPatterns<'a>,
    len: usize,
    index: usize,
}

impl<'a> Iterator for FlatPatternsIterator<'a> {
    type Item = &'a str;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.patterns.data.map_or(None, |fi| {
            if self.index < self.len {
                self.index += 1;
                Some(fi.get(self.index - 1))
            } else {
                None
            }
        })
    }
}

// Implement ExactSizeIterator for FilterPartIterator
impl<'a> ExactSizeIterator for FlatPatternsIterator<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

pub struct FlatNetworkFilterView<'a> {
    pub key: u64,
    pub mask: NetworkFilterMask,
    pub patterns: FlatPatterns<'a>,
    pub modifier_option: Option<&'a str>,
    pub hostname: Option<&'a str>,
    pub opt_domains: Option<&'a [Hash]>,
    pub opt_not_domains: Option<&'a [Hash]>,
    pub tag: Option<&'a str>,
}

impl<'a> From<&'a fb::NetworkFilter<'a>> for FlatNetworkFilterView<'a> {
    #[inline(always)]
    fn from(filter: &'a fb::NetworkFilter<'a>) -> Self {
        let opt_domains = filter.opt_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u64,
                bytes.len() / std::mem::size_of::<u64>(),
            )
        });
        let opt_not_domains = filter.opt_not_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u64,
                bytes.len() / std::mem::size_of::<u64>(),
            )
        });
        Self {
            key: (filter._tab.buf().as_ptr() as *const u64) as u64,
            mask: unsafe { NetworkFilterMask::from_bits_unchecked(filter.mask()) },
            patterns: FlatPatterns {
                data: filter.patterns(),
            },
            modifier_option: filter.modifier_option(),
            hostname: filter.hostname(),
            opt_domains: opt_domains,
            opt_not_domains: opt_not_domains,
            tag: filter.tag(),
        }
    }
}

impl<'a> NetworkMatchable for FlatNetworkFilterView<'a> {
    fn matches(&self, request: &request::Request, regex_manager: &mut RegexManager) -> bool {
        use crate::filters::network_matchers::{check_options, check_pattern, check_domains};
        check_options(
            self.mask,
            request,
        ) && check_domains(
          self.opt_domains.map(|d| d.as_ref()),
          self.opt_not_domains.map(|d| d.as_ref()),
          request,
        ) && check_pattern(
            self.mask,
            self.patterns.iter(),
            self.hostname,
            self.key,
            request,
            regex_manager,
        )
    }

    #[cfg(test)]
    fn matches_test(&self, request: &request::Request) -> bool {
        self.matches(request, &mut RegexManager::default())
    }
}
