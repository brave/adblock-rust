//! Builder for creating flatbuffer with serialized engine.
//! The entry point is `make_flatbuffer`.

use std::collections::HashMap;

use flatbuffers::WIPOffset;

use crate::cosmetic_filter_cache_builder::CosmeticFilterCacheBuilder;
use crate::filters::cosmetic::CosmeticFilter;
use crate::filters::fb_network_builder::{NetworkFilterListBuilder, NetworkRulesBuilder};
use crate::filters::network::NetworkFilter;
use crate::flatbuffers::containers::flat_serialize::{FlatBuilder, FlatSerialize, WIPFlatVec};
use crate::flatbuffers::unsafe_tools::VerifiedFlatbufferMemory;
use crate::utils::Hash;

use super::fb_network::flat::fb;

#[derive(Default)]
pub(crate) struct EngineFlatBuilder<'a> {
    fb_builder: flatbuffers::FlatBufferBuilder<'a>,
    unique_domains_hashes: Vec<Hash>,
    unique_domains_hashes_map: HashMap<Hash, u32>,
}

impl<'a> EngineFlatBuilder<'a> {
    pub fn get_or_insert_unique_domain_hash(&mut self, h: &Hash) -> u32 {
        if let Some(&index) = self.unique_domains_hashes_map.get(h) {
            return index;
        }
        let index = self.unique_domains_hashes.len() as u32;
        self.unique_domains_hashes.push(*h);
        self.unique_domains_hashes_map.insert(*h, index);
        index
    }

    pub fn finish(
        &mut self,
        network_rules: WIPFlatVec<'a, NetworkFilterListBuilder, EngineFlatBuilder<'a>>,
        cosmetic_rules: WIPOffset<fb::CosmeticFilters<'_>>,
        version: u32,
    ) -> VerifiedFlatbufferMemory {
        let unique_domains_hashes =
            Some(self.fb_builder.create_vector(&self.unique_domains_hashes));
        let engine = fb::Engine::create(
            self.raw_builder(),
            &fb::EngineArgs {
                version,
                network_rules: Some(network_rules),
                unique_domains_hashes,
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

pub fn make_flatbuffer(
    network_filters: Vec<NetworkFilter>,
    cosmetic_filters: Vec<CosmeticFilter>,
    optimize: bool,
    version: u32,
) -> VerifiedFlatbufferMemory {
    let mut builder = EngineFlatBuilder::default();
    let network_rules_builder = NetworkRulesBuilder::from_rules(network_filters, optimize);
    let network_rules = FlatSerialize::serialize(network_rules_builder, &mut builder);
    let cosmetic_rules = CosmeticFilterCacheBuilder::from_rules(cosmetic_filters);
    let cosmetic_rules = FlatSerialize::serialize(cosmetic_rules, &mut builder);
    builder.finish(network_rules, cosmetic_rules, version)
}
