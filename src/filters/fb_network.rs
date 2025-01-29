use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::NetworkFilter;
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
