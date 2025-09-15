//! Allows serialization of the adblock engine into a compact binary format, as well as subsequent
//! rapid deserialization back into an engine.
//!
//! In order to support multiple format versions simultaneously, this module wraps around different
//! serialization/deserialization implementations and can automatically dispatch to the appropriate
//! one.
//!
//! The current .dat file format:
//! 1. magic (4 bytes)
//! 2. seahash of the data (8 bytes)
//! 3. data (the rest of the file)

/// Newer formats start with this magic byte sequence.
/// Calculated as the leading 4 bytes of `echo -n 'brave/adblock-rust' | sha512sum`.
const ADBLOCK_RUST_DAT_MAGIC: [u8; 4] = [0xd1, 0xd9, 0x3a, 0xaf];

const HEADER_PREFIX_LENGTH: usize = 12;

#[derive(Debug, PartialEq)]
pub enum DeserializationError {
    BadHeader,
    BadChecksum,
    VersionMismatch(u32),
    FlatBufferParsingError(flatbuffers::InvalidFlatbuffer),
    ValidationError,
}

pub(crate) fn serialize_dat_file(data: &[u8]) -> Vec<u8> {
    let mut serialized = Vec::with_capacity(data.len() + HEADER_PREFIX_LENGTH);
    let hash = seahash::hash(data).to_le_bytes();
    serialized.extend_from_slice(&ADBLOCK_RUST_DAT_MAGIC);
    serialized.extend_from_slice(&hash);
    assert_eq!(serialized.len(), HEADER_PREFIX_LENGTH);

    serialized.extend_from_slice(data);
    serialized
}

pub(crate) fn deserialize_dat_file(serialized: &[u8]) -> Result<&[u8], DeserializationError> {
    if serialized.len() < HEADER_PREFIX_LENGTH || !serialized.starts_with(&ADBLOCK_RUST_DAT_MAGIC) {
        return Err(DeserializationError::BadHeader);
    }
    let data = &serialized[HEADER_PREFIX_LENGTH..];

    // Check the hash to ensure the data isn't corrupted.
    let expected_hash = &serialized[ADBLOCK_RUST_DAT_MAGIC.len()..HEADER_PREFIX_LENGTH];
    if expected_hash != seahash::hash(data).to_le_bytes() {
        println!(
            "Expected hash: {:?}, actual hash: {:?}",
            expected_hash,
            seahash::hash(data).to_le_bytes()
        );
        return Err(DeserializationError::BadChecksum);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_magic_bytes() {
        use sha2::Digest;

        let mut hasher = sha2::Sha512::new();

        hasher.update("brave/adblock-rust");

        let result = hasher.finalize();

        assert!(result.starts_with(&ADBLOCK_RUST_DAT_MAGIC));
    }

    #[test]
    fn serialize_deserialize_test() {
        let data = b"test";
        let serialized = serialize_dat_file(data);
        let deserialized = deserialize_dat_file(&serialized).unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn corrupted_data_test() {
        let data = b"test";
        let serialized = serialize_dat_file(data);
        let mut corrupted_serialized = serialized.clone();
        corrupted_serialized[HEADER_PREFIX_LENGTH] = 0;
        assert_eq!(
            Err(DeserializationError::BadChecksum),
            deserialize_dat_file(&corrupted_serialized)
        );
    }
}
