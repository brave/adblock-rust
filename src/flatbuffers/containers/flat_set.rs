use std::marker::PhantomData;

use crate::flatbuffers::containers::indexable::Indexable;

/// A set-like container that uses flatbuffer references.
/// Provides O(log n) lookup time using binary search on the sorted data.
pub(crate) struct FlatSetView<I, Idx>
where
    Idx: Indexable<I>,
{
    index: Idx,
    _phantom: PhantomData<I>,
}

impl<I, Idx> FlatSetView<I, Idx>
where
    I: Ord + Copy,
    Idx: Indexable<I>,
{
    pub fn new(index: Idx) -> Self {
        Self {
            index,
            _phantom: PhantomData,
        }
    }

    pub fn contains(&self, value: I) -> bool {
        let idx = self.index.partition_point(|x| *x < value);
        idx < self.index.len() && self.index.get(idx) == value
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
