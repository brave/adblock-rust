use std::collections::HashMap;
use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};
use crate::filters::unsafe_tools::{fb_vector_to_slice, VerifiedFlatFilterListMemory};

use crate::network_filter_list::NetworkFilterList;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::{Hash, ShortHash};

#[allow(dead_code, clippy::all, unused_imports, unsafe_code)]
#[path = "../flatbuffers/fb_network_filter_generated.rs"]
pub mod flat;
use flat::fb;

pub(crate) struct FlatNetworkFiltersListBuilder<'a> {
    builder: flatbuffers::FlatBufferBuilder<'a>,
    filters: Vec<fb::NetworkFilter>,

    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u32>,

    // New: strings storage without deduplication
    strings: Vec<String>,
    filter_extras: Vec<WIPOffset<fb::NetworkFilterExtras<'a>>>,
}

impl FlatNetworkFiltersListBuilder<'_> {
    pub fn new() -> Self {
        Self {
            builder: flatbuffers::FlatBufferBuilder::new(),
            filters: vec![],
            unique_domains_hashes: vec![],
            unique_domains_hashes_map: HashMap::new(),
            strings: vec![],
            filter_extras: vec![],
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

    fn add_string(&mut self, s: &str) -> u32 {
        let index = self.strings.len() as u32;
        self.strings.push(s.to_string());
        index + 1  // Use 1-based indexing, 0 means no string
    }

    pub fn add(&mut self, network_filter: &NetworkFilter) -> u32 {
        let opt_domains = network_filter.opt_domains.as_ref().and_then(|v| {
            if v.is_empty() {
                None  // Don't create extras for empty domain lists
            } else {
                let mut o: Vec<u32> = v
                    .iter()
                    .map(|x| self.get_or_insert_unique_domain_hash(x))
                    .collect();
                o.sort_unstable();
                o.dedup();
                Some(self.builder.create_vector(&o))
            }
        });

        let opt_not_domains = network_filter.opt_not_domains.as_ref().and_then(|v| {
            if v.is_empty() {
                None  // Don't create extras for empty domain lists
            } else {
                let mut o: Vec<u32> = v
                    .iter()
                    .map(|x| self.get_or_insert_unique_domain_hash(x))
                    .collect();
                o.sort_unstable();
                o.dedup();
                Some(self.builder.create_vector(&o))
            }
        });

        // Handle any_of_pattern for FilterPart::AnyOf
        let any_of_pattern = if network_filter.filter.iter().len() > 1 {
            let patterns: Vec<String> = network_filter.filter.iter().map(|s| s.to_string()).collect();
            if patterns.is_empty() {
                None  // Don't create extras for empty pattern lists
            } else {
                Some(patterns)
            }
        } else {
            None
        };

        let any_of_pattern_fb = any_of_pattern.as_ref().map(|patterns| {
            let offsets: Vec<WIPOffset<&str>> = patterns
                .iter()
                .map(|s| self.builder.create_string(s))
                .collect();
            self.builder.create_vector(&offsets)
        });

        let raw_line = network_filter
            .raw_line
            .as_ref()
            .and_then(|v| if v.is_empty() { None } else { Some(self.builder.create_string(v.as_str())) });

        let modifier_option = network_filter
            .modifier_option
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { Some(self.builder.create_string(s)) });

        let tag = network_filter
            .tag
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { Some(self.builder.create_string(s)) });

        // Determine hostname_idx
        let hostname_idx = if let Some(ref hostname) = network_filter.hostname {
            self.add_string(hostname)
        } else {
            0
        };

        // Determine single_pattern_idx
        let single_pattern_idx = if network_filter.filter.iter().len() == 1 {
            // FilterPart::Simple - store the single pattern
            let pattern = network_filter.filter.iter().next().unwrap();
            self.add_string(pattern)
        } else if network_filter.filter.iter().len() == 0 {
            // FilterPart::Empty
            0
        } else {
            // FilterPart::AnyOf - use index >= strings.len() to indicate this
            // We'll handle this after we know the final strings length
            u32::MAX // Placeholder, will be updated later
        };

        // Create extras only if we really need them (save memory when possible)
        let extra_idx = if opt_domains.is_some()
            || opt_not_domains.is_some()
            || any_of_pattern_fb.is_some()
            || raw_line.is_some()
            || modifier_option.is_some()
            || tag.is_some() {

            let extras = fb::NetworkFilterExtras::create(
                &mut self.builder,
                &fb::NetworkFilterExtrasArgs {
                    opt_domains,
                    opt_not_domains,
                    any_of_pattern: any_of_pattern_fb,
                    raw_line,
                    modifier_option,
                    tag,
                },
            );
            self.filter_extras.push(extras);
            self.filter_extras.len() as u32
        } else {
            0  // No extras needed, save memory
        };

        // Create the NetworkFilter struct
        let filter = fb::NetworkFilter::new(
            network_filter.mask.bits(),
            hostname_idx,
            single_pattern_idx,
            extra_idx,
        );

        self.filters.push(filter);
        u32::try_from(self.filters.len() - 1).expect("< u32::MAX")
    }

    pub fn finish(
        &mut self,
        mut filter_map: HashMap<ShortHash, Vec<u32>>,
    ) -> VerifiedFlatFilterListMemory {
        println!("extras_count: {:?}", self.filter_extras.len());
        println!("strings_count: {:?}", self.strings.len());
        // Now handle any_of_pattern indices that need to be >= strings.len()
        let strings_base_len = self.strings.len() as u32;
        for (_i, filter) in self.filters.iter_mut().enumerate() {
            if filter.single_pattern_idx() == u32::MAX {
                // This was a placeholder for AnyOf pattern
                // Set it to strings_base_len + index in any_of_pattern
                filter.set_single_pattern_idx(strings_base_len);
            }
        }

        let unique_domains_hashes = self.builder.create_vector(&self.unique_domains_hashes);

        // Create strings vector
        let strings_offsets: Vec<WIPOffset<&str>> = self.strings
            .iter()
            .map(|s| self.builder.create_string(s))
            .collect();
        let strings = self.builder.create_vector(&strings_offsets);

        // Create filter_extras vector
        let filter_extras = self.builder.create_vector(&self.filter_extras);

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
                flat_values.push(self.filters[filter_index as usize]);
            }
        }

        let filter_map_index = self.builder.create_vector(&flat_index);
        let filter_map_values = self.builder.create_vector(&flat_values);

        let storage = fb::NetworkFilterList::create(
            &mut self.builder,
            &fb::NetworkFilterListArgs {
                filter_map_index: Some(filter_map_index),
                filter_map_values: Some(filter_map_values),
                unique_domains_hashes: Some(unique_domains_hashes),
                strings: Some(strings),
                filter_extras: Some(filter_extras),
            },
        );
        self.builder.finish(storage, None);

        // TODO: consider using builder.collapse() to avoid reallocating memory.
        VerifiedFlatFilterListMemory::from_builder(&self.builder)
    }
}
pub(crate) struct FlatPatterns<'a> {
    patterns: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
    single_pattern: Option<&'a str>,
}

impl<'a> FlatPatterns<'a> {
    #[inline(always)]
    pub fn new(
        patterns: Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>,
    ) -> Self {
        Self { patterns, single_pattern: None }
    }

    #[inline(always)]
    pub fn new_single(pattern: &'a str) -> Self {
        Self { patterns: None, single_pattern: Some(pattern) }
    }

    #[inline(always)]
    pub fn iter(&self) -> FlatPatternsIterator {
        FlatPatternsIterator {
            patterns: self,
            len: if let Some(ref patterns) = self.patterns {
                patterns.len()
            } else if self.single_pattern.is_some() {
                1
            } else {
                0
            },
            index: 0,
        }
    }
}

pub(crate) struct FlatPatternsIterator<'a> {
    patterns: &'a FlatPatterns<'a>,
    len: usize,
    index: usize,
}

impl<'a> Iterator for FlatPatternsIterator<'a> {
    type Item = &'a str;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.len {
            let result = if let Some(ref patterns) = self.patterns.patterns {
                Some(patterns.get(self.index))
            } else if let Some(single_pattern) = self.patterns.single_pattern {
                if self.index == 0 {
                    Some(single_pattern)
                } else {
                    None
                }
            } else {
                None
            };
            self.index += 1;
            result
        } else {
            None
        }
    }
}

impl ExactSizeIterator for FlatPatternsIterator<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

pub(crate) struct FlatNetworkFilter<'a> {
    key: u64,
    owner: &'a NetworkFilterList,
    fb_filter: &'a fb::NetworkFilter,

    pub(crate) mask: NetworkFilterMask,
}

impl<'a> FlatNetworkFilter<'a> {
    #[inline(always)]
    pub fn new(
        filter: &'a fb::NetworkFilter,
        index: usize,
        owner: &'a NetworkFilterList,
    ) -> Self {
        let list_address: *const NetworkFilterList = owner as *const NetworkFilterList;

        Self {
            fb_filter: filter,
            key: index as u64 | (((list_address) as u64) << 32),
            mask: NetworkFilterMask::from_bits_retain(filter.mask()),
            owner,
        }
    }

    #[inline(always)]
    pub fn tag(&self) -> Option<&'a str> {
        if self.fb_filter.extra_idx() == 0 {
            None
        } else {
            let extras_list = self.owner.memory.filter_list().filter_extras();
            let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
            extras.tag()
        }
    }

    #[inline(always)]
    pub fn modifier_option(&self) -> Option<String> {
        if self.fb_filter.extra_idx() == 0 {
            None
        } else {
            let extras_list = self.owner.memory.filter_list().filter_extras();
            let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
            extras.modifier_option().map(|s| s.to_string())
        }
    }

    #[inline(always)]
    pub fn include_domains(&self) -> Option<&[u32]> {
        if self.fb_filter.extra_idx() == 0 {
            None
        } else {
            let extras_list = self.owner.memory.filter_list().filter_extras();
            let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
            extras.opt_domains().map(|data| fb_vector_to_slice(data))
        }
    }

    #[inline(always)]
    pub fn exclude_domains(&self) -> Option<&[u32]> {
        if self.fb_filter.extra_idx() == 0 {
            None
        } else {
            let extras_list = self.owner.memory.filter_list().filter_extras();
            let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
            extras.opt_not_domains().map(|data| fb_vector_to_slice(data))
        }
    }

    #[inline(always)]
    pub fn hostname(&self) -> Option<&'a str> {
        if self.mask.is_hostname_anchor() && self.fb_filter.hostname_idx() > 0 {
            let strings = self.owner.memory.filter_list().strings();
            Some(strings.get((self.fb_filter.hostname_idx() - 1) as usize))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn patterns(&self) -> FlatPatterns {
        let single_pattern_idx = self.fb_filter.single_pattern_idx();
        let strings = self.owner.memory.filter_list().strings();

        if single_pattern_idx == 0 {
            // FilterPart::Empty
            FlatPatterns::new(None)
        } else if single_pattern_idx <= strings.len() as u32 {
            // FilterPart::Simple - single pattern (1-based indexing)
            let pattern = strings.get((single_pattern_idx - 1) as usize);
            FlatPatterns::new_single(pattern)
        } else {
            // FilterPart::AnyOf - multiple patterns stored in extras
            if self.fb_filter.extra_idx() == 0 {
                FlatPatterns::new(None)
            } else {
                let extras_list = self.owner.memory.filter_list().filter_extras();
                let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
                FlatPatterns::new(extras.any_of_pattern())
            }
        }
    }

    #[inline(always)]
    pub fn raw_line(&self) -> Option<String> {
        if self.fb_filter.extra_idx() == 0 {
            None
        } else {
            let extras_list = self.owner.memory.filter_list().filter_extras();
            let extras = extras_list.get((self.fb_filter.extra_idx() - 1) as usize);
            extras.raw_line().map(|s| s.to_string())
        }
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
