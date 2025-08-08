//! Contains representations of data from the adblocking engine in a
//! forwards-and-backwards-compatible format, as well as utilities for converting these to and from
//! the actual `Engine` components.
//!
//! Any new fields should be added to the _end_ of both `SerializeFormat` and `DeserializeFormat`.

use rmp_serde as rmps;
use serde::{Deserialize, Serialize};

use crate::filters::unsafe_tools::VerifiedFlatbufferMemory;

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

/// Provides structural aggregration of referenced adblock engine data to allow for allocation-free
/// serialization.
#[derive(Serialize)]
pub(crate) struct SerializeFormat {
    flatbuffer_memory: Vec<u8>,
}

impl SerializeFormat {
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
}

impl DeserializeFormat {
    pub fn deserialize(serialized: &[u8]) -> Result<Self, DeserializationError> {
        let data = super::parse_dat_header(serialized)?;
        let format: Self = rmps::decode::from_read(data)?;
        Ok(format)
    }
}

impl From<&VerifiedFlatbufferMemory> for SerializeFormat {
    fn from(v: &VerifiedFlatbufferMemory) -> Self {
        Self {
            flatbuffer_memory: v.data().to_vec(),
        }
    }
}

impl TryFrom<DeserializeFormat> for VerifiedFlatbufferMemory {
    fn try_from(v: DeserializeFormat) -> Result<Self, Self::Error> {
        let memory = VerifiedFlatbufferMemory::from_raw(v.flatbuffer_memory)
            .map_err(DeserializationError::FlatBufferParsingError)?;

        Ok(memory)
    }

    type Error = DeserializationError;
}
