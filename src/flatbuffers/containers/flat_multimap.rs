use std::marker::PhantomData;

use crate::flatbuffers::containers::indexable::Indexable;
use flatbuffers::{Follow, Vector};

/// A map-like container that uses flatbuffer references.
/// Provides O(log n) lookup time using binary search on the sorted index.
pub(crate) struct FlatMultiMapView<'a, I: Ord + Copy, V, Idx>
where
    Idx: Indexable<I>,
    V: Follow<'a>,
{
    index: Idx,
    values: Vector<'a, V>,
    _phantom: PhantomData<I>,
}

pub(crate) struct FlatMultiMapViewIterator<'a, I: Ord + Copy, V, Idx>
where
    Idx: Indexable<I>,
    V: Follow<'a>,
{
    current_index: usize,
    key: I,
    indexes: Idx,
    values: Vector<'a, V>,
}

impl<'a, I, V, Idx> Iterator for FlatMultiMapViewIterator<'a, I, V, Idx>
where
    I: Ord + Copy,
    V: Follow<'a>,
    Idx: Indexable<I>,
{
    type Item = (usize, <V as Follow<'a>>::Inner);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.indexes.len() {
            if self.indexes.get(self.current_index) != self.key {
                return None;
            }
            let index = self.current_index;
            let filter = self.values.get(self.current_index);
            self.current_index += 1;
            Some((index, filter))
        } else {
            None
        }
    }
}

impl<'a, I: Ord + Copy, V, Idx> FlatMultiMapView<'a, I, V, Idx>
where
    Idx: Indexable<I>,
    V: Follow<'a>,
{
    pub fn new(index: Idx, values: Vector<'a, V>) -> Self {
        debug_assert!(index.len() == values.len());

        Self {
            index,
            values,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self, key: I) -> FlatMultiMapViewIterator<'a, I, V, Idx>
    where
        Idx: Clone,
    {
        let start = self.index.partition_point(|x| *x < key);
        FlatMultiMapViewIterator {
            current_index: start,
            key,
            indexes: self.index.clone(),
            values: self.values,
        }
    }

    #[cfg(test)]
    pub fn total_size(&self) -> usize {
        self.index.len()
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/flatbuffers/containers/flat_multimap.rs"]
mod unit_tests;
