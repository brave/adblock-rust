use std::ops::Range;

use flatbuffers::{Follow, Vector};

use crate::flatbuffers::containers::fb_index::FbIndex;

// Represents sorted sequence to perform the binary search.
pub(crate) trait SortedIndex<I>: FbIndex<I> {
    fn partition_point<F>(&self, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool;

    /// Like [SortedIndex::partition_point], but only searches within `range`. The result is the
    /// same as for the full index as long as the predicate is true for every element before
    /// `range` and false for every element after it.
    fn partition_point_in<F>(&self, range: Range<usize>, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool;
}

// Implementation for slices. Prefer using this with fb_vector_to_slice
// if possible, because it faster than getting values with flatbuffer's
// get method.
impl<I: Ord + Copy> SortedIndex<I> for &[I] {
    #[inline(always)]
    fn partition_point<F>(&self, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool,
    {
        debug_assert!(self.is_sorted());
        <[I]>::partition_point(self, predicate)
    }

    #[inline(always)]
    fn partition_point_in<F>(&self, range: Range<usize>, predicate: F) -> usize
    where
        F: FnMut(&I) -> bool,
    {
        debug_assert!(self.is_sorted());
        range.start + <[I]>::partition_point(&self[range], predicate)
    }
}

// General implementation for flatbuffers::Vector, it uses get to
// obtain values.
impl<'a, T: Follow<'a>> SortedIndex<T::Inner> for Vector<'a, T>
where
    T::Inner: Ord,
{
    fn partition_point<F>(&self, predicate: F) -> usize
    where
        F: FnMut(&T::Inner) -> bool,
    {
        self.partition_point_in(0..self.len(), predicate)
    }

    fn partition_point_in<F>(&self, range: Range<usize>, mut predicate: F) -> usize
    where
        F: FnMut(&T::Inner) -> bool,
    {
        debug_assert!(self.iter().is_sorted());

        let mut left = range.start;
        let mut right = range.end;

        while left < right {
            let mid = left + (right - left) / 2;
            let value = self.get(mid);
            if predicate(&value) {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        left
    }
}
