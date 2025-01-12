use std::collections::HashMap;
use std::vec;

use flatbuffers::WIPOffset;
use itertools::Itertools;

use crate::blocker::NetworkFilterList;
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

    exclude_domains: Vec<Hash>,
    include_domains_map: HashMap<u64, u16>,
}

impl<'a> FlatNetworkFiltersListBuilder<'a> {
    pub fn new() -> Self {
        Self {
            builder: flatbuffers::FlatBufferBuilder::new(),
            filters: vec![],
            exclude_domains: vec![],
            include_domains_map: HashMap::new(),
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

    fn dedup_slice(slice: &mut [u16]) -> &mut [u16] {
        if slice.is_empty() {
            return slice;
        }

        let mut write_index = 1;
        for read_index in 1..slice.len() {
            if slice[read_index] != slice[write_index - 1] {
                slice[write_index] = slice[read_index];
                write_index += 1;
            }
        }
        &mut slice[..write_index]
    }

    pub fn add(&mut self, network_filter: NetworkFilter) -> u32 {
        let mut opt_domains_flat: Option<WIPOffset<flatbuffers::Vector<u16>>> = None;
        let mut opt_not_domains_flat: Option<WIPOffset<flatbuffers::Vector<u16>>> = None;
        if let Some(opt_domains) = network_filter.opt_domains {
          let mut arr: [u16; 10000] = [0; 10000];
          let mut index = 0;
          for domain in opt_domains {
            let len = self.include_domains_map.len();
            let id = self.include_domains_map.entry(domain).or_insert(len as u16);
            arr[index] = *id;
            index += 1;
          }
          // Sort and dedup array:
          arr[..index].sort_unstable();
          let deduped_slice = Self::dedup_slice(&mut arr[..index]);
          let v = self.builder.create_vector_from_iter(deduped_slice.iter());
          opt_domains_flat = Some(v);
        }


        if let Some(exclude_domains) = network_filter.opt_not_domains {
          let mut o: Vec<u16> = exclude_domains
                .into_iter()
                .map(|x| Self::get_or_insert(&mut self.exclude_domains, x))
                .collect();
            o.sort_unstable();
            o.dedup();
            opt_not_domains_flat = Some(self.builder.create_vector(&o));
        }

        let modifier_option = network_filter
            .modifier_option
            .map(|s| self.builder.create_shared_string(&s));

        let hostname = network_filter
            .hostname
            .map(|s| self.builder.create_shared_string(&s));

        let tag = network_filter
            .tag
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
                opt_domains: opt_domains_flat,
                opt_not_domains: opt_not_domains_flat,
                hostname: hostname,
                tag: tag,
            },
        );

        self.filters.push(filter);
        u32::try_from(self.filters.len() - 1).expect("< u32::MAX")
    }

    pub fn finish(&mut self) -> Vec<u8> {
        let filters = self.builder.create_vector(&self.filters);

        let include_domains = self.builder.create_vector_from_iter(
          self.include_domains_map.iter().map(|(k, v)| (v, k)).sorted().map(|(k, v)| *v)
        );
        let exclude_domains = self.builder.create_vector(&self.exclude_domains);

        let storage = fb::NetworkFilterList::create(
            &mut self.builder,
            &&fb::NetworkFilterListArgs {
                global_list: Some(filters),
                unique_include_domains: Some(include_domains),
                unique_exclude_domains: Some(exclude_domains),
            },
        );
        self.builder.finish(storage, None);

        let r = Vec::from(self.builder.finished_data());
        println!(
            "bytes {} i {} e {}",
            r.len(),
            self.include_domains_map.len(),
            self.exclude_domains.len()
        );
        r
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
    fb_filter: &'a fb::NetworkFilter<'a>,
    pub key: u64,
    pub mask: NetworkFilterMask,
    pub tag: Option<&'a str>,
}

impl<'a> From<&'a fb::NetworkFilter<'a>> for FlatNetworkFilterView<'a> {
    #[inline(always)]
    fn from(filter: &'a fb::NetworkFilter<'a>) -> Self {
        /*let opt_domains = filter.opt_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u32,
                bytes.len() / std::mem::size_of::<u32>(),
            )
        });
        let opt_not_domains = filter.opt_not_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u32,
                bytes.len() / std::mem::size_of::<u32>(),
            )
        });*/
        /*Self {
            key: 0,
            mask: unsafe { NetworkFilterMask::from_bits_unchecked(filter.mask()) },
            patterns: FlatPatterns {
                data: filter.patterns(),
            },
            modifier_option: filter.modifier_option(),
            hostname: filter.hostname(),
            opt_domains: opt_domains,
            opt_not_domains: opt_not_domains,
            tag: filter.tag(),
        }*/
        Self {
            fb_filter: filter,
            key: 0,
            mask: unsafe { NetworkFilterMask::from_bits_unchecked(filter.mask()) },
            tag: filter.tag(),
        }
    }
}

struct CheckOptionsParams {
    pub mask: NetworkFilterMask,
}

impl<'a> From<&'a FlatNetworkFilterView<'a>> for CheckOptionsParams {
    #[inline(always)]
    fn from(filter: &'a FlatNetworkFilterView<'a>) -> Self {
        Self { mask: filter.mask }
    }
}

struct CheckPatternsParams<'a> {
    pub patterns: FlatPatterns<'a>,
    pub hostname: Option<&'a str>,
}

impl<'a> From<&'a FlatNetworkFilterView<'a>> for CheckPatternsParams<'a> {
    #[inline(always)]
    fn from(filter: &'a FlatNetworkFilterView<'a>) -> Self {
        Self {
            patterns: FlatPatterns {
                data: filter.fb_filter.patterns(),
            },
            hostname: if filter.mask.is_hostname_anchor() {
                filter.fb_filter.hostname()
            } else {
                None
            },
        }
    }
}

impl<'a> NetworkMatchable for FlatNetworkFilterView<'a> {
    fn matches(
        &self,
        request: &request::Request,
        network_list: &NetworkFilterList,
        regex_manager: &mut RegexManager,
    ) -> bool {
        use crate::filters::network_matchers::{
            check_excluded_domains, check_included_domains, check_options, check_pattern,
        };
        let cop = CheckOptionsParams::from(self);
        if !check_options(cop.mask, request) {
            return false;
        }
        let opt_not_domains = self.fb_filter.opt_not_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u16,
                bytes.len() / std::mem::size_of::<u16>(),
            )
        });
        if !check_excluded_domains(opt_not_domains, request, &network_list.exclude_domains_map) {
            return false;
        }
        let opt_domains = self.fb_filter.opt_domains().map(|domains| unsafe {
            let bytes = domains.bytes();
            std::slice::from_raw_parts(
                bytes.as_ptr() as *const u16,
                bytes.len() / std::mem::size_of::<u16>(),
            )
        });
        if !check_included_domains(opt_domains, request, &network_list.include_domains_map) {
            return false;
        }
        let cpp = CheckPatternsParams::from(self);
        if !check_pattern(
            self.mask,
            cpp.patterns.iter(),
            cpp.hostname,
            self.key,
            request,
            regex_manager,
        ) {
            return false;
        }
        true
    }

    #[cfg(test)]
    fn matches_test(&self, request: &request::Request) -> bool {
        self.matches(request, &mut RegexManager::default())
    }
}
