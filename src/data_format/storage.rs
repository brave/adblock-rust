//! Contains representations of data from the adblocking engine in a
//! forwards-and-backwards-compatible format, as well as utilities for converting these to and from
//! the actual `Engine` components.
//!
//! Any new fields should be added to the _end_ of both `SerializeFormat` and `DeserializeFormat`.

use std::collections::{HashMap, HashSet};

use rmp_serde as rmps;
use serde::{Deserialize, Serialize};

use crate::cosmetic_filter_cache::{
    CosmeticFilterCache, CosmeticFilterNotProtoFields, HostnameRuleDb, ProceduralOrActionFilter,
};
use crate::filters::fb_network::{FilterDataContext, FilterDataContextRef};
use crate::filters::unsafe_tools::VerifiedFlatbufferMemory;
use crate::utils::Hash;

use super::utils::{stabilize_hashmap_serialization, stabilize_hashset_serialization};
use super::{DeserializationError, SerializationError};

/// Each variant describes a single rule that is specific to a particular hostname.
#[derive(Clone, Debug, Deserialize, Serialize)]
enum LegacySpecificFilterType {
    Hide(String),
    Unhide(String),
    Style(String, String),
    UnhideStyle(String, String),
    ScriptInject(String),
    UnhideScriptInject(String),
}

#[derive(Deserialize, Serialize, Default)]
pub(crate) struct LegacyHostnameRuleDb {
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    db: HashMap<Hash, Vec<LegacySpecificFilterType>>,
}

impl From<&HostnameRuleDb> for LegacyHostnameRuleDb {
    fn from(v: &HostnameRuleDb) -> Self {
        let mut db = HashMap::<Hash, Vec<LegacySpecificFilterType>>::new();
        for (hash, bin) in v.hide.0.iter() {
            for f in bin {
                db.entry(*hash)
                    .and_modify(|v| v.push(LegacySpecificFilterType::Hide(f.to_owned())))
                    .or_insert_with(|| vec![LegacySpecificFilterType::Hide(f.to_owned())]);
            }
        }
        for (hash, bin) in v.unhide.0.iter() {
            for f in bin {
                db.entry(*hash)
                    .and_modify(|v| v.push(LegacySpecificFilterType::Unhide(f.to_owned())))
                    .or_insert_with(|| vec![LegacySpecificFilterType::Unhide(f.to_owned())]);
            }
        }
        for (hash, bin) in v.inject_script.0.iter() {
            for (f, _mask) in bin {
                db.entry(*hash)
                    .and_modify(|v| v.push(LegacySpecificFilterType::ScriptInject(f.to_owned())))
                    .or_insert_with(|| vec![LegacySpecificFilterType::ScriptInject(f.to_owned())]);
            }
        }
        for (hash, bin) in v.uninject_script.0.iter() {
            for f in bin {
                db.entry(*hash)
                    .and_modify(|v| {
                        v.push(LegacySpecificFilterType::UnhideScriptInject(f.to_owned()))
                    })
                    .or_insert_with(|| {
                        vec![LegacySpecificFilterType::UnhideScriptInject(f.to_owned())]
                    });
            }
        }
        for (hash, bin) in v.procedural_action.0.iter() {
            for f in bin {
                if let Ok(f) = serde_json::from_str::<ProceduralOrActionFilter>(f) {
                    if let Some((selector, style)) = f.as_css() {
                        db.entry(*hash)
                            .and_modify(|v| {
                                v.push(LegacySpecificFilterType::Style(
                                    selector.clone(),
                                    style.clone(),
                                ))
                            })
                            .or_insert_with(|| {
                                vec![LegacySpecificFilterType::Style(selector, style)]
                            });
                    }
                }
            }
        }
        for (hash, bin) in v.procedural_action_exception.0.iter() {
            for f in bin {
                if let Ok(f) = serde_json::from_str::<ProceduralOrActionFilter>(f) {
                    if let Some((selector, style)) = f.as_css() {
                        db.entry(*hash)
                            .and_modify(|v| {
                                v.push(LegacySpecificFilterType::UnhideStyle(
                                    selector.to_owned(),
                                    style.to_owned(),
                                ))
                            })
                            .or_insert_with(|| {
                                vec![LegacySpecificFilterType::UnhideStyle(
                                    selector.to_owned(),
                                    style.to_owned(),
                                )]
                            });
                    }
                }
            }
        }
        LegacyHostnameRuleDb { db }
    }
}

impl From<LegacyHostnameRuleDb> for HostnameRuleDb {
    fn from(val: LegacyHostnameRuleDb) -> Self {
        use crate::cosmetic_filter_cache::HostnameFilterBin;

        let mut hide = HostnameFilterBin::default();
        let mut unhide = HostnameFilterBin::default();
        let mut procedural_action = HostnameFilterBin::default();
        let mut procedural_action_exception = HostnameFilterBin::default();
        let mut inject_script = HostnameFilterBin::default();
        let mut uninject_script = HostnameFilterBin::default();

        for (hash, bin) in val.db.into_iter() {
            for rule in bin.into_iter() {
                match rule {
                    LegacySpecificFilterType::Hide(s) => hide.insert(&hash, s),
                    LegacySpecificFilterType::Unhide(s) => unhide.insert(&hash, s),
                    LegacySpecificFilterType::Style(s, st) => procedural_action
                        .insert_procedural_action_filter(
                            &hash,
                            &ProceduralOrActionFilter::from_css(s, st),
                        ),
                    LegacySpecificFilterType::UnhideStyle(s, st) => procedural_action_exception
                        .insert_procedural_action_filter(
                            &hash,
                            &ProceduralOrActionFilter::from_css(s, st),
                        ),
                    LegacySpecificFilterType::ScriptInject(s) => {
                        inject_script.insert(&hash, (s, Default::default()))
                    }
                    LegacySpecificFilterType::UnhideScriptInject(s) => {
                        uninject_script.insert(&hash, s)
                    }
                }
            }
        }
        HostnameRuleDb {
            hide,
            unhide,
            inject_script,
            uninject_script,
            procedural_action,
            procedural_action_exception,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct LegacyRedirectResource {
    pub content_type: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub(crate) struct LegacyRedirectResourceStorage {
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    pub resources: HashMap<String, LegacyRedirectResource>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct LegacyScriptletResource {
    scriptlet: String,
}

#[derive(Default, Deserialize, Serialize)]
pub(crate) struct LegacyScriptletResourceStorage {
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    resources: HashMap<String, LegacyScriptletResource>,
}

/// Provides structural aggregration of referenced adblock engine data to allow for allocation-free
/// serialization.
#[derive(Serialize)]
pub(crate) struct SerializeFormat<'a> {
    flatbuffer_memory: Vec<u8>,

    resources: LegacyRedirectResourceStorage,

    #[serde(serialize_with = "stabilize_hashset_serialization")]
    simple_id_rules: &'a HashSet<String>,
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    complex_class_rules: &'a HashMap<String, Vec<String>>,
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    complex_id_rules: &'a HashMap<String, Vec<String>>,

    specific_rules: LegacyHostnameRuleDb,

    #[serde(serialize_with = "stabilize_hashset_serialization")]
    misc_generic_selectors: &'a HashSet<String>,

    scriptlets: LegacyScriptletResourceStorage,

    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    procedural_action: &'a HashMap<Hash, Vec<String>>,
    #[serde(serialize_with = "stabilize_hashmap_serialization")]
    procedural_action_exception: &'a HashMap<Hash, Vec<String>>,
}

impl SerializeFormat<'_> {
    pub fn serialize(&self) -> Result<Vec<u8>, SerializationError> {
        let mut output = super::ADBLOCK_RUST_DAT_MAGIC.to_vec();
        output.push(super::ADBLOCK_RUST_DAT_VERSION);
        rmps::encode::write(&mut output, &self)?;
        Ok(output)
    }
}

/// Structural representation of adblock engine data that can be built up from deserialization and
/// used directly to construct new `Engine` components without unnecessary allocation.
#[derive(Deserialize)]
pub(crate) struct DeserializeFormat {
    flatbuffer_memory: Vec<u8>,

    _resources: LegacyRedirectResourceStorage,

    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: LegacyHostnameRuleDb,

    misc_generic_selectors: HashSet<String>,

    _scriptlets: LegacyScriptletResourceStorage,

    #[serde(default)]
    procedural_action: HashMap<Hash, Vec<String>>,
    #[serde(default)]
    procedural_action_exception: HashMap<Hash, Vec<String>>,
}

impl DeserializeFormat {
    pub fn deserialize(serialized: &[u8]) -> Result<Self, DeserializationError> {
        let data = super::parse_dat_header(serialized)?;
        let format: Self = rmps::decode::from_read(data)?;
        Ok(format)
    }
}

impl<'a> From<(&'a FilterDataContext, &'a CosmeticFilterCache)> for SerializeFormat<'a> {
    fn from(v: (&'a FilterDataContext, &'a CosmeticFilterCache)) -> Self {
        let (context, cfc) = v;
        Self {
            flatbuffer_memory: context.memory.data().to_vec(),

            resources: LegacyRedirectResourceStorage::default(),

            simple_id_rules: &cfc.simple_id_rules,
            complex_class_rules: &cfc.complex_class_rules,
            complex_id_rules: &cfc.complex_id_rules,

            specific_rules: (&cfc.specific_rules).into(),

            misc_generic_selectors: &cfc.misc_generic_selectors,

            scriptlets: LegacyScriptletResourceStorage::default(),

            procedural_action: &cfc.specific_rules.procedural_action.0,
            procedural_action_exception: &cfc.specific_rules.procedural_action_exception.0,
        }
    }
}

impl TryFrom<DeserializeFormat> for (FilterDataContextRef, CosmeticFilterCache) {
    fn try_from(v: DeserializeFormat) -> Result<Self, Self::Error> {
        use crate::cosmetic_filter_cache::HostnameFilterBin;
        use crate::filters::fb_network::FilterDataContext;

        let mut specific_rules: HostnameRuleDb = v.specific_rules.into();
        specific_rules.procedural_action = HostnameFilterBin(v.procedural_action);
        specific_rules.procedural_action_exception =
            HostnameFilterBin(v.procedural_action_exception);

        let memory = VerifiedFlatbufferMemory::from_raw(v.flatbuffer_memory)
            .map_err(DeserializationError::FlatBufferParsingError)?;

        let filter_data_context = FilterDataContext::new(memory);

        let cosmetic_cache = CosmeticFilterCache::from_context(
            filter_data_context.clone(),
            CosmeticFilterNotProtoFields {
                simple_id_rules: v.simple_id_rules,
                complex_class_rules: v.complex_class_rules,
                complex_id_rules: v.complex_id_rules,
                specific_rules,
                misc_generic_selectors: v.misc_generic_selectors,
            },
        );

        Ok((filter_data_context, cosmetic_cache))
    }

    type Error = DeserializationError;
}
