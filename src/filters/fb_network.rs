use std::collections::HashMap;
use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};
use crate::network_filter_list::FlatNetworkFilterList;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::Hash;

#[allow(dead_code, unused_imports, unsafe_code)]
#[path = "../flat/fb_network_filter_generated.rs"]
pub mod flat;
use flat::fb;

pub struct FlatNetworkFiltersListBuilder<'a> {
    builder: flatbuffers::FlatBufferBuilder<'a>,
    filters: Vec<WIPOffset<fb::NetworkFilter<'a>>>,

    unique_domains: Vec<Hash>,
}

impl<'a> FlatNetworkFiltersListBuilder<'a> {
    pub fn new() -> Self {
        Self {
            builder: flatbuffers::FlatBufferBuilder::new(),
            filters: vec![],
            unique_domains: vec![],
        }
    }

    fn get_or_insert(arr: &mut Vec<Hash>, h: Hash) -> u16 {
        if let Some(index) = arr.iter().position(|&x| x == h) {
            u16::try_from(index).expect("< u16 max")
        } else {
            arr.push(h);
            u16::try_from(arr.len() - 1).expect("< u16 max")
        }
    }

    pub fn add(&mut self, network_filter: &NetworkFilter) -> u32 {
        let opt_domains = network_filter.opt_domains.as_ref().map(|v| {
            let mut o: Vec<u16> = v
                .into_iter()
                .map(|x| Self::get_or_insert(&mut self.unique_domains, *x))
                .collect();
            o.sort_unstable();
            o.dedup();
            self.builder.create_vector(&o)
        });

        let opt_not_domains = network_filter.opt_not_domains.as_ref().map(|v| {
            let mut o: Vec<u16> = v
                .into_iter()
                .map(|x| Self::get_or_insert(&mut self.unique_domains, *x))
                .collect();
            o.sort_unstable();
            o.dedup();
            self.builder.create_vector(&o)
        });

        let modifier_option = network_filter
            .modifier_option
            .as_ref()
            .map(|s| self.builder.create_shared_string(&s));

        let hostname = network_filter
            .hostname
            .as_ref()
            .map(|s| self.builder.create_shared_string(&s));

        let tag = network_filter
            .tag
            .as_ref()
            .map(|s| self.builder.create_shared_string(&s));

        let patterns = if network_filter.filter.iter().len() > 0 {
            let offsets: Vec<WIPOffset<&str>> = network_filter
                .filter
                .iter()
                .map(|s| self.builder.create_shared_string(s))
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

        let unique_domains = self.builder.create_vector(&self.unique_domains);

        let storage = fb::NetworkFilterList::create(
            &mut self.builder,
            &&fb::NetworkFilterListArgs {
                network_filters: Some(filters),
                unique_domains_hashes: Some(unique_domains),
            },
        );
        self.builder.finish(storage, None);

        let binary = Vec::from(self.builder.finished_data());
        binary
    }
}
pub struct FlatPatterns<'a> {
    patterns: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
}

impl<'a> FlatPatterns<'a> {
    #[inline(always)]
    pub fn new(
        patterns: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
    ) -> Self {
        Self { patterns }
    }

    #[inline(always)]
    pub fn iter(&self) -> FlatPatternsIterator {
        FlatPatternsIterator {
            patterns: self,
            len: self.patterns.map_or(0, |d| d.len()),
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
        self.patterns.patterns.map_or(None, |fi| {
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

pub struct FlatNetworkFilter<'a> {
    key: u64,
    owner: &'a FlatNetworkFilterList,
    fb_filter: &'a fb::NetworkFilter<'a>,

    pub mask: NetworkFilterMask,
}

impl<'a> FlatNetworkFilter<'a> {
    #[inline(always)]
    pub fn new(
        filter: &'a fb::NetworkFilter<'a>,
        index: u32,
        owner: &'a FlatNetworkFilterList,
    ) -> Self {
        let list_address: *const FlatNetworkFilterList = owner as *const FlatNetworkFilterList;

        Self {
            fb_filter: filter,
            key: index as u64 | (((list_address) as u64) << 32),
            mask: unsafe { NetworkFilterMask::from_bits_unchecked(filter.mask()) },
            owner: owner,
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
    pub fn include_domains(&self) -> Option<&[u16]> {
        self.fb_filter.opt_domains().map(|data| {
            let bytes = data.bytes();
            unsafe {
                std::slice::from_raw_parts(
                    bytes.as_ptr() as *const u16,
                    bytes.len() / std::mem::size_of::<u16>(),
                )
            }
        })
    }

    #[inline(always)]
    pub fn exclude_domains(&self) -> Option<&[u16]> {
        self.fb_filter.opt_not_domains().map(|data| {
            let bytes = data.bytes();
            unsafe {
                std::slice::from_raw_parts(
                    bytes.as_ptr() as *const u16,
                    bytes.len() / std::mem::size_of::<u16>(),
                )
            }
        })
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
    pub fn patterns(&self) -> FlatPatterns {
        FlatPatterns::new(self.fb_filter.patterns())
    }
}

impl<'a> NetworkFilterMaskHelper for FlatNetworkFilter<'a> {
    #[inline]
    fn has_flag(&self, v: NetworkFilterMask) -> bool {
        self.mask.contains(v)
    }
}

impl<'a> NetworkMatchable for FlatNetworkFilter<'a> {
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
            &self.owner.domain_hashes_mapping,
        ) {
            return false;
        }
        if !check_excluded_domains_mapped(
            self.exclude_domains(),
            request,
            &self.owner.domain_hashes_mapping,
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
