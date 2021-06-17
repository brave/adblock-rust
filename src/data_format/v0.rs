//! Contains representations of data from the adblocking engine in a
//! forwards-and-backwards-compatible format, as well as utilities for converting these to and from
//! the actual `Engine` components.
//!
//! Any new fields should be added to the _end_ of both `SerializeFormat` and `DeserializeFormat`.

use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};
use rmp_serde as rmps;

use crate::blocker::{Blocker, NetworkFilterList};
use crate::resources::{RedirectResourceStorage, ScriptletResourceStorage};
use crate::filters::network::NetworkFilter;
use crate::cosmetic_filter_cache::{CosmeticFilterCache, HostnameRuleDb};

use super::SerializationError;
use super::DeserializationError;

/// Provides structural aggregration of referenced adblock engine data to allow for allocation-free
/// serialization.
#[derive(Serialize)]
pub struct SerializeFormat<'a> {
    csp: &'a NetworkFilterList,
    exceptions: &'a NetworkFilterList,
    importants: &'a NetworkFilterList,
    redirects: &'a NetworkFilterList,
    filters_tagged: &'a NetworkFilterList,
    filters: &'a NetworkFilterList,
    generic_hide: &'a NetworkFilterList,

    tagged_filters_all: &'a Vec<NetworkFilter>,

    enable_optimizations: bool,

    resources: &'a RedirectResourceStorage,

    simple_class_rules: &'a HashSet<String>,
    simple_id_rules: &'a HashSet<String>,
    complex_class_rules: &'a HashMap<String, Vec<String>>,
    complex_id_rules: &'a HashMap<String, Vec<String>>,

    specific_rules: &'a HostnameRuleDb,

    misc_generic_selectors: &'a HashSet<String>,

    scriptlets: &'a ScriptletResourceStorage,
}

impl<'a> SerializeFormat<'a> {
    pub fn serialize(&self) -> Result<Vec<u8>, SerializationError> {
        let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 11, 24);
        rmps::encode::write(&mut writer, &0u64)?;
        rmps::encode::write(&mut writer, &self)?;
        let compressed = writer.into_inner();
        Ok(compressed)
    }
}

/// Structural representation of adblock engine data that can be built up from deserialization and
/// used directly to construct new `Engine` components without unnecessary allocation.
#[derive(Deserialize)]
pub struct DeserializeFormat {
    csp: NetworkFilterList,
    exceptions: NetworkFilterList,
    importants: NetworkFilterList,
    redirects: NetworkFilterList,
    filters_tagged: NetworkFilterList,
    filters: NetworkFilterList,
    generic_hide: NetworkFilterList,

    tagged_filters_all: Vec<NetworkFilter>,

    enable_optimizations: bool,

    resources: RedirectResourceStorage,

    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: HostnameRuleDb,

    misc_generic_selectors: HashSet<String>,

    scriptlets: ScriptletResourceStorage,
}

impl DeserializeFormat {
    pub fn deserialize(serialized: &[u8]) -> Result<Self, DeserializationError> {
        let mut decompressor = brotli::Decompressor::new(serialized, 4096);
        let version: usize = rmps::decode::from_read(&mut decompressor)?;
        assert_eq!(version, 0);
        let format: Self = rmps::decode::from_read(&mut decompressor)?;
        Ok(format)
    }
}

impl<'a> From<(&'a Blocker, &'a CosmeticFilterCache)> for SerializeFormat<'a> {
    fn from(v: (&'a Blocker, &'a CosmeticFilterCache)) -> Self {
        let (blocker, cfc) = v;
        Self {
            csp: &blocker.csp,
            exceptions: &blocker.exceptions,
            importants: &blocker.importants,
            redirects: &blocker.redirects,
            filters_tagged: &blocker.filters_tagged,
            filters: &blocker.filters,
            generic_hide: &blocker.generic_hide,

            tagged_filters_all: &blocker.tagged_filters_all,

            enable_optimizations: blocker.enable_optimizations,

            resources: &blocker.resources,

            simple_class_rules: &cfc.simple_class_rules,
            simple_id_rules: &cfc.simple_id_rules,
            complex_class_rules: &cfc.complex_class_rules,
            complex_id_rules: &cfc.complex_id_rules,

            specific_rules: &cfc.specific_rules,

            misc_generic_selectors: &cfc.misc_generic_selectors,

            scriptlets: &cfc.scriptlets,
        }
    }
}

impl Into<(Blocker, CosmeticFilterCache)> for DeserializeFormat {
    fn into(self) -> (Blocker, CosmeticFilterCache) {
        (Blocker {
            csp: self.csp,
            exceptions: self.exceptions,
            importants: self.importants,
            redirects: self.redirects,
            filters_tagged: self.filters_tagged,
            filters: self.filters,
            generic_hide: self.generic_hide,

            tags_enabled: Default::default(),
            tagged_filters_all: self.tagged_filters_all,

            hot_filters: Default::default(),

            enable_optimizations: self.enable_optimizations,

            resources: self.resources,
            #[cfg(feature = "object-pooling")]
            pool: Default::default(),

        }, CosmeticFilterCache {
            simple_class_rules: self.simple_class_rules,
            simple_id_rules: self.simple_id_rules,
            complex_class_rules: self.complex_class_rules,
            complex_id_rules: self.complex_id_rules,

            specific_rules: self.specific_rules,

            misc_generic_selectors: self.misc_generic_selectors,

            scriptlets: self.scriptlets,
        })
    }
}
