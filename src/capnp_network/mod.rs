use std::collections::HashMap;
use std::vec;

use capnp::message::{Builder, HeapAllocator};
use capnp::serialize;

use crate::filters::network::{
    NetworkFilter, NetworkFilterMask, NetworkFilterMaskHelper, NetworkMatchable,
};

use crate::network_filter_list::NetworkFilterList;
use crate::regex_manager::RegexManager;
use crate::request::Request;
use crate::utils::Hash;

// Re-export the generated code at the crate level to fix module path issues
pub mod network_filter_capnp {
    include!(concat!(env!("OUT_DIR"), "/network_filter_capnp.rs"));
}

use network_filter_capnp::{network_filter, network_filter_list};

pub(crate) struct CapnpNetworkFiltersListBuilder {
    message: Builder<HeapAllocator>,
    filters: Vec<NetworkFilter>, // Store actual filters instead of serialized data
    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u16>,
}

impl CapnpNetworkFiltersListBuilder {
    pub fn new() -> Self {
        Self {
            message: Builder::new_default(),
            filters: vec![],
            unique_domains_hashes: vec![],
            unique_domains_hashes_map: HashMap::new(),
        }
    }

    fn get_or_insert_unique_domain_hash(&mut self, h: &Hash) -> u16 {
        if let Some(&index) = self.unique_domains_hashes_map.get(h) {
            return index;
        }
        let index = self.unique_domains_hashes.len() as u16;
        self.unique_domains_hashes.push(*h);
        self.unique_domains_hashes_map.insert(*h, index);
        return index;
    }

    pub fn add(&mut self, network_filter: &NetworkFilter) -> u32 {
        // Just store the filter, we'll serialize everything at once in finish()
        self.filters.push(network_filter.clone());

        // Still need to track unique domain hashes
        if let Some(ref domains) = network_filter.opt_domains {
            for domain in domains {
                self.get_or_insert_unique_domain_hash(domain);
            }
        }
        if let Some(ref not_domains) = network_filter.opt_not_domains {
            for domain in not_domains {
                self.get_or_insert_unique_domain_hash(domain);
            }
        }

        u32::try_from(self.filters.len() - 1).expect("< u32::MAX")
    }

    pub fn finish(&mut self) -> Vec<u8> {
        let mut list_builder = self.message.init_root::<network_filter_list::Builder>();

        // Create the filters list directly
        let mut filters_builder = list_builder.reborrow().init_network_filters(self.filters.len() as u32);

        for (i, network_filter) in self.filters.iter().enumerate() {
            let mut filter_builder = filters_builder.reborrow().get(i as u32);

            filter_builder.set_mask(network_filter.mask.bits());

            if let Some(ref domains) = network_filter.opt_domains {
                let mut domain_indices: Vec<u16> = domains
                    .iter()
                    .map(|x| self.unique_domains_hashes_map[x])
                    .collect();
                domain_indices.sort_unstable();
                domain_indices.dedup();

                let mut opt_domains_builder = filter_builder.reborrow().init_opt_domains(domain_indices.len() as u32);
                for (j, &domain_idx) in domain_indices.iter().enumerate() {
                    opt_domains_builder.set(j as u32, domain_idx);
                }
            }

            if let Some(ref not_domains) = network_filter.opt_not_domains {
                let mut domain_indices: Vec<u16> = not_domains
                    .iter()
                    .map(|x| self.unique_domains_hashes_map[x])
                    .collect();
                domain_indices.sort_unstable();
                domain_indices.dedup();

                let mut opt_not_domains_builder = filter_builder.reborrow().init_opt_not_domains(domain_indices.len() as u32);
                for (j, &domain_idx) in domain_indices.iter().enumerate() {
                    opt_not_domains_builder.set(j as u32, domain_idx);
                }
            }

            if network_filter.filter.iter().len() > 0 {
                let mut patterns_builder = filter_builder.reborrow().init_patterns(network_filter.filter.iter().len() as u32);
                for (j, pattern) in network_filter.filter.iter().enumerate() {
                    patterns_builder.set(j as u32, pattern);
                }
            }

            if let Some(ref modifier_option) = network_filter.modifier_option {
                filter_builder.set_modifier_option(modifier_option);
            }

            if let Some(ref hostname) = network_filter.hostname {
                filter_builder.set_hostname(hostname);
            }

            if let Some(ref tag) = network_filter.tag {
                filter_builder.set_tag(tag);
            }

            // Only set raw_line if it's actually present (debug mode)
            if let Some(ref raw_line) = network_filter.raw_line {
                filter_builder.set_raw_line(raw_line.as_str());
            }
        }

        let mut unique_domains_builder = list_builder.reborrow().init_unique_domains_hashes(self.unique_domains_hashes.len() as u32);
        for (i, &hash) in self.unique_domains_hashes.iter().enumerate() {
            unique_domains_builder.set(i as u32, hash);
        }

        // Use packed encoding to reduce size significantly
        let mut packed_buffer = Vec::new();
        capnp::serialize_packed::write_message(&mut packed_buffer, &self.message).unwrap();
        packed_buffer
    }
}

pub(crate) struct CapnpPatterns<'a> {
    patterns: Option<capnp::text_list::Reader<'a>>,
}

impl<'a> CapnpPatterns<'a> {
    #[inline(always)]
    pub fn new(patterns: Option<capnp::text_list::Reader<'a>>) -> Self {
        Self { patterns }
    }

    #[inline(always)]
    pub fn iter(&self) -> CapnpPatternsIterator {
        CapnpPatternsIterator {
            patterns: self,
            len: self.patterns.map_or(0, |d| d.len()),
            index: 0,
        }
    }
}

pub(crate) struct CapnpPatternsIterator<'a> {
    patterns: &'a CapnpPatterns<'a>,
    len: u32,
    index: u32,
}

impl<'a> Iterator for CapnpPatternsIterator<'a> {
    type Item = Result<&'a str, capnp::Error>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.patterns.patterns.map_or(None, |fi| {
            if self.index < self.len {
                let result = fi.get(self.index).map_err(capnp::Error::from).and_then(|r| {
                    r.to_str().map_err(capnp::Error::from)
                });
                self.index += 1;
                Some(result)
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
        // Create a unique key similar to FlatBuffers implementation
        let list_address: *const NetworkFilterList = owner as *const NetworkFilterList;
        let key = index as u64 | (((list_address) as u64) << 32);

        Self {
            key,
            owner,
            capnp_filter: filter,
            mask,
        }
    }

    #[inline(always)]
    pub fn get_id(&self) -> u64 {
        self.key
    }

    pub fn tag(&self) -> Option<&'a str> {
        if self.capnp_filter.has_tag() {
            self.capnp_filter.get_tag().ok()
                .and_then(|r| r.to_str().ok())
        } else {
            None
        }
    }

    pub fn modifier_option(&self) -> Option<String> {
        if self.capnp_filter.has_modifier_option() {
            self.capnp_filter.get_modifier_option().ok()
                .and_then(|r| r.to_str().ok())
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    pub fn hostname(&self) -> Option<&'a str> {
        if self.mask.is_hostname_anchor() && self.capnp_filter.has_hostname() {
            self.capnp_filter.get_hostname().ok()
                .and_then(|r| r.to_str().ok())
        } else {
            None
        }
    }

    pub fn patterns(&self) -> CapnpPatterns {
        let patterns = if self.capnp_filter.has_patterns() {
            self.capnp_filter.get_patterns().ok()
        } else {
            None
        };
        CapnpPatterns::new(patterns)
    }

    pub fn raw_line(&self) -> Option<String> {
        if self.capnp_filter.has_raw_line() {
            self.capnp_filter.get_raw_line().ok()
                .and_then(|r| r.to_str().ok())
                .map(|s| s.to_string())
        } else {
            None
        }
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
        use crate::filters::network_matchers::{check_options, check_pattern};

        if !check_options(self.mask, request) {
            return false;
        }

        // Check included domains efficiently
        if self.capnp_filter.has_opt_domains() {
            if let Ok(domains) = self.capnp_filter.get_opt_domains() {
                if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
                    let mut found = false;
                    for i in 0..domains.len() {
                        let domain_idx = domains.get(i);
                        // Find the hash corresponding to this domain index
                        if let Some(&hash) = self.owner.unique_domains_hashes_map.iter()
                            .find(|(_, &idx)| idx == domain_idx)
                            .map(|(hash, _)| hash) {
                            if source_hashes.iter().any(|h| *h == hash) {
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        return false;
                    }
                }
            }
        }

        // Check excluded domains efficiently
        if self.capnp_filter.has_opt_not_domains() {
            if let Ok(domains) = self.capnp_filter.get_opt_not_domains() {
                if let Some(source_hashes) = request.source_hostname_hashes.as_ref() {
                    for i in 0..domains.len() {
                        let domain_idx = domains.get(i);
                        // Find the hash corresponding to this domain index
                        if let Some(&hash) = self.owner.unique_domains_hashes_map.iter()
                            .find(|(_, &idx)| idx == domain_idx)
                            .map(|(hash, _)| hash) {
                            if source_hashes.iter().any(|h| *h == hash) {
                                return false;
                            }
                        }
                    }
                }
            }
        }

        // Convert patterns iterator to collect strings for pattern matching
        let patterns_binding = self.patterns();
        let patterns: Result<Vec<&str>, _> = patterns_binding.iter().collect();
        match patterns {
            Ok(pattern_vec) => {
                let hostname = self.hostname();
                check_pattern(
                    self.mask,
                    pattern_vec.into_iter(),
                    hostname,
                    self.key,
                    request,
                    regex_manager,
                )
            }
            Err(_) => false,
        }
    }

    #[cfg(test)]
    fn matches_test(&self, request: &Request) -> bool {
        self.matches(request, &mut RegexManager::default())
    }
}
