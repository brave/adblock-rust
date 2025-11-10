//! Builder for creating flatbuffer with serialized engine.

use std::collections::HashMap;

use flatbuffers::WIPOffset;

use crate::filters::fb_network_builder::NetworkFilterListBuilder;
use crate::flatbuffers::containers::flat_serialize::{FlatBuilder, FlatSerialize, WIPFlatVec};
use crate::flatbuffers::unsafe_tools::VerifiedFlatbufferMemory;
use crate::utils::Hash;

use super::flat::fb;

#[derive(Clone, Default)]
pub(crate) struct ShareableString {
    index: Option<usize>,
}

#[derive(Default)]
pub(crate) struct EngineFlatBuilder<'a> {
    fb_builder: flatbuffers::FlatBufferBuilder<'a>,
    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u32>,
    unique_domains_strings: Vec<String>,
    shared_strings: Vec<WIPOffset<&'a str>>,
    shared_strings_original: Vec<String>,
}

impl<'a> EngineFlatBuilder<'a> {
    pub fn get_or_insert_unique_domain(&mut self, h: &Hash, domain_str: &str) -> u32 {
        if let Some(&index) = self.unique_domains_hashes_map.get(h) {
            return index;
        }
        let index = self.unique_domains_hashes.len() as u32;
        self.unique_domains_hashes.push(*h);
        self.unique_domains_strings.push(domain_str.to_string());
        self.unique_domains_hashes_map.insert(*h, index);
        index
    }

    pub fn add_shareable_string(&mut self, s: &str) -> ShareableString {
        let wip_offset = self.fb_builder.create_string(s);
        self.shared_strings.push(wip_offset);
        self.shared_strings_original.push(s.to_string());
        ShareableString {
            index: Some(self.shared_strings.len() - 1),
        }
    }

    pub fn finish(
        &mut self,
        network_rules: WIPFlatVec<'a, NetworkFilterListBuilder, EngineFlatBuilder<'a>>,
        cosmetic_rules: WIPOffset<fb::CosmeticFilters<'_>>,
    ) -> VerifiedFlatbufferMemory {
        let unique_domains_hashes =
            Some(self.fb_builder.create_vector(&self.unique_domains_hashes));

        // Create vector of domain strings
        let domain_string_offsets: Vec<_> = self
            .unique_domains_strings
            .iter()
            .map(|s| self.fb_builder.create_string(s))
            .collect();
        let unique_domains_strings = Some(self.fb_builder.create_vector(&domain_string_offsets));

        let engine = fb::Engine::create(
            self.raw_builder(),
            &fb::EngineArgs {
                network_rules: Some(network_rules),
                unique_domains_hashes,
                unique_domains_strings,
                cosmetic_filters: Some(cosmetic_rules),
            },
        );
        self.raw_builder().finish(engine, None);
        VerifiedFlatbufferMemory::from_builder(self.raw_builder())
    }
}

impl<'a> FlatBuilder<'a> for EngineFlatBuilder<'a> {
    fn create_string(&mut self, s: &str) -> WIPOffset<&'a str> {
        self.fb_builder.create_string(s)
    }

    fn raw_builder(&mut self) -> &mut flatbuffers::FlatBufferBuilder<'a> {
        &mut self.fb_builder
    }
}

impl<'a> FlatSerialize<'a, EngineFlatBuilder<'a>> for ShareableString {
    type Output = WIPOffset<&'a str>;
    fn serialize(value: Self, builder: &mut EngineFlatBuilder<'a>) -> Self::Output {
        if let Some(index) = value.index {
            builder.shared_strings[index]
        } else {
            builder.raw_builder().create_shared_string("")
        }
    }
}
