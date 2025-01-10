use std::vec;

use flatbuffers::WIPOffset;

use crate::filters::network::{NetworkFilter, NetworkFilterMask};
use crate::regex_manager::RegexManager;
use crate::request::{self};
use crate::utils::Hash;
use flatbuffers::{Table, Follow};

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

impl<'a> FlatNetworkFilterView<'a> {
  #[inline]
  unsafe fn get_from_vtable<T: Follow<'a> + 'a>(
      table: &Table<'a>,
      offset: usize,
      default: Option<T::Inner>,
  ) -> Option<T::Inner> {
      if offset == 0 {
          return default;
      }
      Some(<T>::follow(table.buf(), table.loc() + offset))
  }
}

impl<'a> From<fb::NetworkFilter<'a>> for FlatNetworkFilterView<'a> {
    #[inline(always)]
    fn from(filter: fb::NetworkFilter<'a>) -> Self {

              // Safety:
        // Created from valid Table for this object
        // which contains valid values in these slots
        let vtable = filter._tab.vtable();

        let mask_offset = vtable.get(fb::NetworkFilter::VT_MASK) as usize;
        let opt_domains_offset = vtable.get(fb::NetworkFilter::VT_OPT_DOMAINS) as usize;
        let opt_not_domains_offset = vtable.get(fb::NetworkFilter::VT_OPT_NOT_DOMAINS) as usize;
        let patterns_offset = vtable.get(fb::NetworkFilter::VT_PATTERNS) as usize;
        let modifier_option_offset = vtable.get(fb::NetworkFilter::VT_MODIFIER_OPTION) as usize;
        let hostname_offset = vtable.get(fb::NetworkFilter::VT_HOSTNAME) as usize;
        let tag_offset = vtable.get(fb::NetworkFilter::VT_TAG) as usize;

        let mask = unsafe { FlatNetworkFilterView::get_from_vtable::<u32>(&filter._tab, mask_offset, Some(0)).unwrap() };
        let opt_domains_raw = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, u64>>>(&filter._tab, opt_domains_offset, None) };
        let opt_not_domains_raw = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, u64>>>(&filter._tab, opt_not_domains_offset, None) };
        let patterns = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>>(&filter._tab, patterns_offset, None) };
        let modifier_option = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<&str>>(&filter._tab, modifier_option_offset, None) };
        let hostname = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<&str>>(&filter._tab, hostname_offset, None) };
        let tag = unsafe { FlatNetworkFilterView::get_from_vtable::<flatbuffers::ForwardsUOffset<&str>>(&filter._tab, tag_offset, None) };

        Self {
            key: (filter._tab.buf().as_ptr() as *const u64) as u64,
            mask: unsafe { NetworkFilterMask::from_bits_unchecked(mask) },
            patterns: FlatPatterns { data: patterns },
            modifier_option,
            hostname,
            opt_domains: opt_domains_raw.map(|domains| unsafe {
                let bytes = domains.bytes();
                std::slice::from_raw_parts(
                    bytes.as_ptr() as *const u64,
                    bytes.len() / std::mem::size_of::<u64>(),
                )
            }),
            opt_not_domains: opt_not_domains_raw.map(|domains| unsafe {
                let bytes = domains.bytes();
                std::slice::from_raw_parts(
                    bytes.as_ptr() as *const u64,
                    bytes.len() / std::mem::size_of::<u64>(),
                )
            }),
            tag,
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
