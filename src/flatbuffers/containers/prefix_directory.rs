//! A small in-memory acceleration structure for sorted arrays of hash keys.

use std::ops::Range;

/// The directory has `2^floor(log2(n / MIN_KEYS_PER_BUCKET))` buckets for `n` keys, so until
/// [MAX_PREFIX_BITS] is reached, maps with at least 4 keys average 2-4 keys per bucket, at a cost of
/// 1-2 bytes per key. A denser directory was measured to be only marginally faster on the Brave
/// lists, while doubling the memory cost.
const MIN_KEYS_PER_BUCKET: usize = 2;

/// Upper bound on the number of prefix bits, to keep the size of a single directory bounded
/// (about 256 KiB). Only reached for maps with at least 2^17 keys.
const MAX_PREFIX_BITS: u32 = 16;

/// Maps the top bits of a `u32` key to the range of a sorted key array that can contain it.
///
/// The network filter maps are sorted arrays of [crate::utils::ShortHash] keys, which are
/// uniformly distributed. A plain binary search over the largest of them (>100k keys for the
/// default Brave lists) takes ~17 dependent loads, and most lookups are for tokens that aren't
/// present at all. Bucketing by the top bits of the key replaces the first ~15 of those steps
/// with a single directory lookup, leaving a search over a few adjacent keys.
///
/// The directory is derived from the key array at load time, so the serialized format is
/// unchanged and the result of a lookup is identical to a binary search over the full array.
pub(crate) struct PrefixDirectory {
    /// `key >> shift` is the bucket index. The shift is done on a `u64` so that a 0-bit directory
    /// (`shift == 32`) is valid.
    shift: u32,
    /// `offsets[b]..offsets[b + 1]` is the range of keys in bucket `b`.
    offsets: Box<[u32]>,
}

impl PrefixDirectory {
    /// Builds a directory over `sorted_keys`. If the keys are not sorted (i.e. a malformed
    /// serialized engine), all ranges are still in bounds, but lookups may miss keys, just as a
    /// binary search would.
    pub fn new(sorted_keys: &[u32]) -> Self {
        debug_assert!(sorted_keys.is_sorted());
        // Always true for FlatBuffers vectors, whose lengths are `u32`.
        debug_assert!(u32::try_from(sorted_keys.len()).is_ok());

        let bits = (sorted_keys.len() / MIN_KEYS_PER_BUCKET)
            .checked_ilog2()
            .unwrap_or(0)
            .min(MAX_PREFIX_BITS);
        let shift = u32::BITS - bits;
        let buckets = 1usize << bits;

        // Count the keys in each bucket, then turn the counts into start offsets. Unlike a merge
        // over the sorted keys, this has no data-dependent branches.
        let mut offsets = vec![0u32; buckets + 1];
        for &key in sorted_keys {
            offsets[Self::bucket_of(shift, key) + 1] += 1;
        }
        for bucket in 0..buckets {
            offsets[bucket + 1] += offsets[bucket];
        }

        Self {
            shift,
            offsets: offsets.into_boxed_slice(),
        }
    }

    #[inline(always)]
    fn bucket_of(shift: u32, key: u32) -> usize {
        ((key as u64) >> shift) as usize
    }

    /// Returns the range of the sorted key array which contains every occurrence of `key`, if
    /// any. In general, the position of the first key not less than `key` in the whole array is
    /// within the returned range or at its end, so a binary search over the range returns the
    /// same position as one over the whole array.
    #[inline(always)]
    pub fn range(&self, key: u32) -> Range<usize> {
        let bucket = Self::bucket_of(self.shift, key);
        self.offsets[bucket] as usize..self.offsets[bucket + 1] as usize
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/flatbuffers/containers/prefix_directory.rs"]
mod unit_tests;
