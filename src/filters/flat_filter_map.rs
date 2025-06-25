//! Holds the implementation of [FlatFilterMap].

use flatbuffers::{Follow, ForwardsUOffset, Vector, WIPOffset};
use std::collections::{HashMap, HashSet};

/// A map-like container that uses flatbuffer references.
/// Provides O(log n) lookup time using binary search on the sorted index.
pub(crate) struct FlatFilterMap<'a, I: Ord + Copy, V> {
    index: &'a [I],
    values: Vector<'a, ForwardsUOffset<V>>,
}

/// Iterator over NetworkFilter objects from [FlatFilterMap]
pub(crate) struct FlatFilterMapIterator<'a, I: Ord + Copy, V> {
    current_index: usize,
    key: I,
    indexes: &'a [I],
    values: Vector<'a, ForwardsUOffset<V>>,
}

impl<'a, I, V> Iterator for FlatFilterMapIterator<'a, I, V>
where
    I: Ord + Copy,
    V: Follow<'a>,
{
    type Item = (usize, <V as Follow<'a>>::Inner);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.indexes.len() {
            if self.indexes[self.current_index] != self.key {
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

impl<'a, I: Ord + Copy, V> FlatFilterMap<'a, I, V> {
    /// Construct [FlatFilterMap] from two vectors:
    /// - index: sorted array of keys
    /// - values: array of values, same length as index
    pub fn new(index: &'a [I], values: Vector<'a, ForwardsUOffset<V>>) -> Self {
        // Sanity check the size are equal. Note: next() will handle |values| correctly.
        debug_assert!(index.len() == values.len());

        debug_assert!(index.is_sorted());

        Self { index, values }
    }

    /// Get an iterator over NetworkFilter objects with the given hash key.
    pub fn get(&self, key: I) -> FlatFilterMapIterator<'a, I, V> {
        let start = self.index.partition_point(|x| *x < key);
        FlatFilterMapIterator {
            current_index: start,
            key,
            indexes: self.index,
            values: self.values,
        }
    }
}

impl<I: Ord + Copy, V> FlatFilterMap<'_, I, V> {
    #[cfg(test)]
    pub fn total_size(&self) -> usize {
        self.index.len()
    }
}

pub struct SerializedFlatMap<'a, I: Ord + Copy + flatbuffers::Push, V: flatbuffers::Push> {
    pub indexes: WIPOffset<Vector<'a, I::Output>>,
    pub values: WIPOffset<Vector<'a, V::Output>>,
}

pub struct SerializedFlatSet<'a, I: Ord + Copy + flatbuffers::Push> {
    pub keys: WIPOffset<Vector<'a, I::Output>>,
}

fn write_sorted_entries<'a, I: Ord + Copy + flatbuffers::Push, V: flatbuffers::Push>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    entries: Vec<(I, Vec<V>)>,
) -> SerializedFlatMap<'a, I, V> {
    let mut indexes = Vec::with_capacity(entries.len());
    let mut values = Vec::with_capacity(entries.len());

    for (k, mv) in entries.iter() {
        for v in mv.iter() {
            indexes.push(k);
            values.push(v);
        }
    }

    SerializedFlatMap {
        indexes: builder.create_vector(&indexes),
        values: builder.create_vector(&values),
    }
}

pub(crate) fn write_hash_multi_map<'a, I: Ord + Copy + flatbuffers::Push, V: flatbuffers::Push>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    map: HashMap<I, Vec<V>>,
) -> SerializedFlatMap<'a, I, V> {
    // Convert `map` to a sorted vector of (key, value).
    let mut entries: Vec<_> = map.into_iter().collect();
    entries.sort_unstable_by_key(|(k, _)| *k);

    write_sorted_entries(builder, entries)
}

pub(crate) fn write_hash_set<'a, I: Ord + Copy + flatbuffers::Push>(
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    set: HashSet<I>,
) -> SerializedFlatSet<'a, I> {
    let mut keys = set.into_iter().collect::<Vec<_>>();
    keys.sort_unstable();
    SerializedFlatSet {
        keys: builder.create_vector(&keys),
    }
}
