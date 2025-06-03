/// Unsafe utility functions for working with flatbuffers and other low-level operations.
use crate::filters::fb_network::flat::fb;

/// Converts a flatbuffers Vector to a slice.
///
/// # Safety
/// This function uses unsafe code to convert flatbuffer vector bytes to a slice.
/// It assumes the vector data is properly aligned and sized for type T.
#[inline(always)]
pub fn fb_vector_to_slice<'a, T>(vector: flatbuffers::Vector<'a, T>) -> &'a [T] {
    let bytes = vector.bytes();
    assert!(bytes.len() % std::mem::size_of::<T>() == 0);
    assert!(bytes.as_ptr() as usize % std::mem::align_of::<T>() == 0);
    unsafe {
        std::slice::from_raw_parts(
            bytes.as_ptr() as *const T,
            bytes.len() / std::mem::size_of::<T>(),
        )
    }
}

const ALIGNMENT: usize = 8;

// A safe wrapper around the flatbuffer data.
// It could be constructed from raw data (includes the flatbuffer verification)
// or from a builder that have just been used to construct the flatbuffer
// Invariants:
// 1. self.data() is properly verified flatbuffer contains FilterList.
// 2. self.data() is aligned to ALIGNMENT bytes.
// This is necessary to fb_vector_to_slice works for [u8]
pub(crate) struct VerifiedFlatFilterListMemory {
    // The buffer containing the flatbuffer data.
    // The flatbuffer data start MUST be aligned to ALIGNMENT bytes.
    raw_data: Vec<u8>,

    // The offset of the start of the flatbuffer data in `raw_data`.
    start: usize,
}

impl VerifiedFlatFilterListMemory {
    pub(crate) fn from_raw(data: Vec<u8>) -> Result<Self, flatbuffers::InvalidFlatbuffer> {
        let is_aligned = data.as_ptr() as usize % ALIGNMENT == 0;
        let memory = if is_aligned {
            // A fast track for 64 bit machines, no need to allocate new memory.
            Self {
                raw_data: data,
                start: 0,
            }
        } else {
            Self::from_u8_slice_unsafe(&data)
        };

        // Verify that the data is a valid flatbuffer.
        let _ = fb::root_as_network_filter_list(&memory.data())?;

        Ok(memory)
    }

    fn from_u8_slice_unsafe(slice: &[u8]) -> Self {
        let mut data = vec![0; ALIGNMENT];
        let start = ALIGNMENT - (data.as_ptr() as usize % ALIGNMENT);
        data.resize(start, 0);

        data.reserve(slice.len());
        data.extend_from_slice(slice);

        Self {
            raw_data: data,
            start: start,
        }
    }

    // Creates a new VerifiedFlatFilterListMemory from a builder.
    // The builder must contains a valid FilterList.
    pub(crate) fn from_builder(builder: &flatbuffers::FlatBufferBuilder<'_>) -> Self {
        Self::from_u8_slice_unsafe(builder.finished_data())
    }

    pub(crate) fn filter_list<'a>(&'a self) -> fb::NetworkFilterList<'a> {
        return unsafe { fb::root_as_network_filter_list_unchecked(&self.data()) };
    }

    pub fn data(&self) -> &[u8] {
        &self.raw_data[self.start..]
    }
}
