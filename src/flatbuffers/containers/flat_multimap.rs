use std::marker::PhantomData;

use crate::flatbuffers::containers::sorted_index::SortedIndex;
use flatbuffers::{Follow, Vector};

/// A map-like container that uses flatbuffer references.
/// Provides O(log n) lookup time using binary search on the sorted index.
/// I is a key type, Keys is specific container of keys, &[I] for fast indexing (u32, u64)
/// and flatbuffers::Vector<I> if there is no conversion from Vector (str) to slice.
pub(crate) struct FlatMultiMapView<'a, I: Ord, V, Keys>
where
    Keys: SortedIndex<I>,
    V: Follow<'a>,
{
    keys: Keys,
    values: Vector<'a, V>,
    _phantom: PhantomData<I>,
}

impl<'a, I: Ord + Copy, V, Keys> FlatMultiMapView<'a, I, V, Keys>
where
    Keys: SortedIndex<I> + Clone,
    V: Follow<'a>,
{
    pub fn new(keys: Keys, values: Vector<'a, V>) -> Self {
        debug_assert!(keys.len() == values.len());

        Self {
            keys,
            values,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self, key: I) -> FlatMultiMapViewIterator<'a, I, V, Keys> {
        FlatMultiMapViewIterator {
            index: self.keys.partition_point(|x| *x < key),
            key,
            keys: self.keys.clone(), // Cloning is 3-4% faster than & in benchmarks
            values: self.values,
        }
    }

    #[cfg(test)]
    pub fn total_size(&self) -> usize {
        self.keys.len()
    }
}

pub(crate) struct FlatMultiMapViewIterator<'a, I: Ord + Copy, V, Keys>
where
    Keys: SortedIndex<I>,
    V: Follow<'a>,
{
    index: usize,
    key: I,
    keys: Keys,
    values: Vector<'a, V>,
}

impl<'a, I, V, Keys> Iterator for FlatMultiMapViewIterator<'a, I, V, Keys>
where
    I: Ord + Copy,
    V: Follow<'a>,
    Keys: SortedIndex<I>,
{
    type Item = (usize, <V as Follow<'a>>::Inner);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.keys.len() && self.keys.get(self.index) == self.key {
            self.index += 1;
            Some((self.index - 1, self.values.get(self.index - 1)))
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/flatbuffers/containers/flat_multimap.rs"]
mod unit_tests;
