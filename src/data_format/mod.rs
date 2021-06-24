//! Allows serialization of the adblock engine into a compact binary format, as well as subsequent
//! rapid deserialization back into an engine.
//!
//! In order to support multiple format versions simultaneously, this module wraps around different
//! serialization/deserialization implementations and can automatically dispatch to the appropriate
//! one.

mod legacy;
mod v0;

use crate::blocker::Blocker;
use crate::cosmetic_filter_cache::CosmeticFilterCache;

/// Provides structural aggregration of referenced adblock engine data to allow for allocation-free
/// serialization.
///
/// Note that this does not implement `Serialize` directly, as it is composed of parts which must
/// be serialized independently. Instead, use the `serialize` method.
pub enum SerializeFormat<'a> {
    Legacy(legacy::SerializeFormat<'a>),
    V0(v0::SerializeFormat<'a>),
}

#[derive(Debug)]
pub enum SerializationError {
    RmpSerdeError(rmp_serde::encode::Error),
    GzError(std::io::Error),
}

/// Since two different versions of `rmp-serde` are being used, errors must be converted to a
/// single implementation.
impl From<rmp_serde_legacy::encode::Error> for SerializationError {
    fn from(e: rmp_serde_legacy::encode::Error) -> Self {
        use rmp_serde_legacy::encode::Error as LegacyEncodeError;
        use rmp_serde::encode::Error as EncodeError;

        let new_error = match e {
            LegacyEncodeError::InvalidValueWrite(e) => EncodeError::InvalidValueWrite(e),
            LegacyEncodeError::UnknownLength => EncodeError::UnknownLength,
            LegacyEncodeError::DepthLimitExceeded => EncodeError::DepthLimitExceeded,
            LegacyEncodeError::Syntax(e) => EncodeError::Syntax(e),
        };
        Self::RmpSerdeError(new_error)
    }
}

impl From<rmp_serde::encode::Error> for SerializationError {
    fn from(e: rmp_serde::encode::Error) -> Self { Self::RmpSerdeError(e) }
}

impl From<std::io::Error> for SerializationError {
    fn from(e: std::io::Error) -> Self { Self::GzError(e) }
}

impl<'a> SerializeFormat<'a> {
    pub fn serialize(&self) -> Result<Vec<u8>, SerializationError> {
        match self {
            Self::Legacy(v) => v.serialize(),
            Self::V0(v) => v.serialize(),
        }
    }
}

impl<'a> From<(&'a Blocker, &'a CosmeticFilterCache)> for SerializeFormat<'a> {
    fn from((blocker, cfc): (&'a Blocker, &'a CosmeticFilterCache)) -> Self {
        // For now, only support writing the legacy format, even though reading the newer format is
        // supported. Version 0.4 of the crate will write only the V0 format.
        Self::Legacy(legacy::SerializeFormat::from((blocker, cfc)))
    }
}

/// Structural representation of adblock engine data that can be built up from deserialization and
/// used directly to construct new `Engine` components without unnecessary allocation.
///
/// Note that this does not implement `Deserialize` directly, as it is composed of parts which must
/// be deserialized independently. Instead, use the `deserialize` method.
pub enum DeserializeFormat {
    Legacy(legacy::DeserializeFormat),
    V0(v0::DeserializeFormat),
}

#[derive(Debug)]
pub enum DeserializationError {
    RmpSerdeError(rmp_serde::decode::Error),
    UnsupportedFormatVersion(u64),
}

/// Since two different versions of `rmp-serde` are being used, errors must be converted to a
/// single implementation.
impl From<rmp_serde_legacy::decode::Error> for DeserializationError {
    fn from(e: rmp_serde_legacy::decode::Error) -> Self {
        use rmp_serde_legacy::decode::Error as LegacyDecodeError;
        use rmp_serde::decode::Error as DecodeError;

        let new_error = match e {
            LegacyDecodeError::InvalidMarkerRead(e) => DecodeError::InvalidMarkerRead(e),
            LegacyDecodeError::InvalidDataRead(e) => DecodeError::InvalidDataRead(e),
            LegacyDecodeError::TypeMismatch(m) => DecodeError::TypeMismatch(m),
            LegacyDecodeError::OutOfRange => DecodeError::OutOfRange,
            LegacyDecodeError::LengthMismatch(l) => DecodeError::LengthMismatch(l),
            LegacyDecodeError::Uncategorized(e) => DecodeError::Uncategorized(e),
            LegacyDecodeError::Syntax(e) => DecodeError::Syntax(e),
            LegacyDecodeError::Utf8Error(e) => DecodeError::Utf8Error(e),
            LegacyDecodeError::DepthLimitExceeded => DecodeError::DepthLimitExceeded,
        };
        Self::RmpSerdeError(new_error)
    }
}

impl From<rmp_serde::decode::Error> for DeserializationError {
    fn from(e: rmp_serde::decode::Error) -> Self { Self::RmpSerdeError(e) }
}

impl DeserializeFormat {
    pub fn deserialize(serialized: &[u8]) -> Result<Self, DeserializationError> {
        // A brotli stream can technically be created with a valid gzip header.
        // However, the probability of also generating a conformant MessagePack-encoded
        // legacy::DeserializeFormat is astronomically low, to the point where the legacy and v0+
        // formats can be considered distinct.
        //
        // For correctness, both formats must be attempted before deserialization is considered
        // "failed". However, in the interest of performance, we can use the exact 10-byte header
        // sequence generated by the uncustomized flate2 GzEncoder as a heuristic for which method
        // is most likely to succeed.
        /// adblock-rust has always used flate2 1.0.x, which has never changed the header sequence
        /// from these 10 bits when the GzEncoder is left uncustomized.
        const FLATE2_GZ_HEADER_BYTES: [u8; 10] = [31, 139, 8, 0, 0, 0, 0, 0, 0, 255];
        // Check for the exact 10-byte header sequence generated by the uncustomized flate2
        // GzEncoder.
        if serialized.starts_with(&FLATE2_GZ_HEADER_BYTES) {
            // If present, attempt to deserialize the legacy format,
            match legacy::DeserializeFormat::deserialize(serialized) {
                Ok(d) => Ok(Self::Legacy(d)),
                Err(e) => {
                    // but fallback to checking for a brotli stream with a version number if that
                    // is unsuccessful.
                    let mut decompressor = brotli::Decompressor::new(serialized, 4096);
                    if let Ok(version) = rmp_serde::decode::from_read::<_, u64>(&mut decompressor) {
                        match version {
                            0 => Ok(Self::V0(v0::DeserializeFormat::deserialize(serialized)?)),
                            v => Err(DeserializationError::UnsupportedFormatVersion(v)),
                        }
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            // If the sequence is missing, attempt to decode a version number from the buffer as a
            // brotli stream.
            let mut decompressor = brotli::Decompressor::new(serialized, 4096);
            if let Ok(version) = rmp_serde::decode::from_read::<_, u64>(&mut decompressor) {
                match version {
                    0 => Ok(Self::V0(v0::DeserializeFormat::deserialize(serialized)?)),
                    v => Err(DeserializationError::UnsupportedFormatVersion(v)),
                }
            } else {
                // If the data still couldn't be deserialized correctly, and gzip hasn't been attempted
                // yet, try it.
                Ok(Self::Legacy(legacy::DeserializeFormat::deserialize(serialized)?))
            }
        }
    }
}

impl Into<(Blocker, CosmeticFilterCache)> for DeserializeFormat {
    fn into(self) -> (Blocker, CosmeticFilterCache) {
        match self {
            Self::Legacy(v) => v.into(),
            Self::V0(v) => v.into(),
        }
    }
}
