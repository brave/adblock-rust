//! Contains representations of data from the adblocking engine in a
//! forwards-and-backwards-compatible format, as well as utilities for converting these to and from
//! the actual `Engine` components.
//!
//! The format itself is split into two parts for historical reasons. Any new fields should be
//! added to the _end_ of both `SerializeFormatRest` and `DeserializeFormatRest`.

use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;
use rmp_serde as rmps;

use crate::blocker::{Blocker, NetworkFilterList};
use crate::resources::{RedirectResourceStorage, ScriptletResourceStorage};
use crate::filters::network::NetworkFilter;
use crate::cosmetic_filter_cache::{CosmeticFilterCache, HostnameRuleDb};
use crate::utils::is_eof_error;

/// Provides structural aggregration of referenced adblock engine data to allow for allocation-free
/// serialization.
///
/// Note that this does not implement `Serialize` directly, as it is composed of two parts which
/// must be serialized independently. Instead, use the `serialize` method.
pub struct SerializeFormat<'a> {
    part1: SerializeFormatPt1<'a>,
    rest: SerializeFormatRest<'a>,
}

#[derive(Debug)]
pub enum SerializationError {
    RmpSerdeError(rmps::encode::Error),
    GzError(std::io::Error),
}

impl From<rmps::encode::Error> for SerializationError {
    fn from(e: rmps::encode::Error) -> Self { Self::RmpSerdeError(e) }
}

impl From<std::io::Error> for SerializationError {
    fn from(e: std::io::Error) -> Self { Self::GzError(e) }
}

impl<'a> SerializeFormat<'a> {
    pub fn serialize(&self) -> Result<Vec<u8>, SerializationError> {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        rmps::encode::write(&mut gz, &self.part1)?;
        rmps::encode::write(&mut gz, &self.rest)?;
        let compressed = gz.finish()?;
        Ok(compressed)
    }
}

#[derive(Serialize)]
struct SerializeFormatPt1<'a> {
    csp: &'a NetworkFilterList,
    exceptions: &'a NetworkFilterList,
    importants: &'a NetworkFilterList,
    redirects: &'a NetworkFilterList,
    filters_tagged: &'a NetworkFilterList,
    filters: &'a NetworkFilterList,

    tagged_filters_all: &'a Vec<NetworkFilter>,

    _debug: bool,
    enable_optimizations: bool,

    // This field exists for backwards compatibility only.
    _unused: bool,
    // This field exists for backwards compatibility only, and *must* be true.
    _unused2: bool,

    resources: &'a RedirectResourceStorage,
}

#[derive(Serialize)]
struct SerializeFormatRest<'a> {
    simple_class_rules: &'a HashSet<String>,
    simple_id_rules: &'a HashSet<String>,
    complex_class_rules: &'a HashMap<String, Vec<String>>,
    complex_id_rules: &'a HashMap<String, Vec<String>>,

    specific_rules: &'a HostnameRuleDb,

    misc_generic_selectors: &'a HashSet<String>,

    scriptlets: &'a ScriptletResourceStorage,

    generic_hide: &'a NetworkFilterList,
}

/// Structural representation of adblock engine data that can be built up from deserialization and
/// used directly to construct new `Engine` components without unnecessary allocation.
///
/// Note that this does not implement `Deserialize` directly, as it is composed of two parts which
/// must be deserialized independently. Instead, use the `deserialize` method.
pub struct DeserializeFormat {
    part1: DeserializeFormatPart1,
    rest: DeserializeFormatRest,
}

#[derive(Debug)]
pub enum DeserializationError {
    RmpSerdeError(rmps::decode::Error),
}

impl From<rmps::decode::Error> for DeserializationError {
    fn from(e: rmps::decode::Error) -> Self { Self::RmpSerdeError(e) }
}

impl DeserializeFormat {
    pub fn deserialize(serialized: &[u8]) -> Result<Self, DeserializationError> {
        let mut gz = GzDecoder::new(serialized);
        let part1: DeserializeFormatPart1 = rmps::decode::from_read(&mut gz)?;
        let rest = match rmps::decode::from_read(&mut gz) {
            Ok(rest) => rest,
            Err(ref e) if is_eof_error(e) => Default::default(),
            Err(e) => return Err(DeserializationError::RmpSerdeError(e)),
        };
        Ok(Self { part1, rest })
    }
}

#[derive(Deserialize)]
struct DeserializeFormatPart1 {
    csp: NetworkFilterList,
    exceptions: NetworkFilterList,
    importants: NetworkFilterList,
    redirects: NetworkFilterList,
    filters_tagged: NetworkFilterList,
    filters: NetworkFilterList,

    tagged_filters_all: Vec<NetworkFilter>,

    debug: bool,
    enable_optimizations: bool,

    // This field exists for backwards compatibility only.
    _unused: bool,
    // This field exists for backwards compatibility only, and *must* be true.
    _unused2: bool,

    #[serde(default)]
    resources: RedirectResourceStorage,
}

/// Any fields added to this must include the `#[serde(default)]` annotation, or another serde
/// annotation that will allow the format to gracefully handle missing fields when deserializing
/// from older versions of the format.
#[derive(Deserialize, Default)]
struct DeserializeFormatRest {
    #[serde(default)]
    simple_class_rules: HashSet<String>,
    #[serde(default)]
    simple_id_rules: HashSet<String>,
    #[serde(default)]
    complex_class_rules: HashMap<String, Vec<String>>,
    #[serde(default)]
    complex_id_rules: HashMap<String, Vec<String>>,

    #[serde(default)]
    specific_rules: HostnameRuleDb,

    #[serde(default)]
    misc_generic_selectors: HashSet<String>,

    #[serde(default)]
    scriptlets: ScriptletResourceStorage,

    #[serde(default)]
    generic_hide: NetworkFilterList,
}

impl<'a> From<(&'a Blocker, &'a CosmeticFilterCache)> for SerializeFormat<'a> {
    fn from(v: (&'a Blocker, &'a CosmeticFilterCache)) -> Self {
        let (blocker, cfc) = v;
        Self {
            part1: SerializeFormatPt1 {
                csp: &blocker.csp,
                exceptions: &blocker.exceptions,
                importants: &blocker.importants,
                redirects: &blocker.redirects,
                filters_tagged: &blocker.filters_tagged,
                filters: &blocker.filters,

                tagged_filters_all: &blocker.tagged_filters_all,

                _debug: true,
                enable_optimizations: blocker.enable_optimizations,
                _unused: true,
                _unused2: true,

                resources: &blocker.resources,
            },
            rest: SerializeFormatRest {
                simple_class_rules: &cfc.simple_class_rules,
                simple_id_rules: &cfc.simple_id_rules,
                complex_class_rules: &cfc.complex_class_rules,
                complex_id_rules: &cfc.complex_id_rules,

                specific_rules: &cfc.specific_rules,

                misc_generic_selectors: &cfc.misc_generic_selectors,

                scriptlets: &cfc.scriptlets,

                generic_hide: &blocker.generic_hide,
            },
        }
    }
}

impl From<DeserializeFormat> for (Blocker, CosmeticFilterCache) {
    fn from(format: DeserializeFormat) -> Self {
        (Blocker {
            csp: format.part1.csp,
            exceptions: format.part1.exceptions,
            importants: format.part1.importants,
            redirects: format.part1.redirects,
            filters_tagged: format.part1.filters_tagged,
            filters: format.part1.filters,

            tags_enabled: Default::default(),
            tagged_filters_all: format.part1.tagged_filters_all,

            hot_filters: Default::default(),

            enable_optimizations: format.part1.enable_optimizations,

            resources: format.part1.resources,
            #[cfg(feature = "object-pooling")]
            pool: Default::default(),

            generic_hide: format.rest.generic_hide,
        }, CosmeticFilterCache {
            simple_class_rules: format.rest.simple_class_rules,
            simple_id_rules: format.rest.simple_id_rules,
            complex_class_rules: format.rest.complex_class_rules,
            complex_id_rules: format.rest.complex_id_rules,

            specific_rules: format.rest.specific_rules,

            misc_generic_selectors: format.rest.misc_generic_selectors,

            scriptlets: format.rest.scriptlets,
        })
    }
}
