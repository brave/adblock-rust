//! Holds the implementation of [FlatFilterMap].

use flatbuffers::{Follow, ForwardsUOffset, Vector, WIPOffset};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use crate::filters::unsafe_tools::fb_vector_to_slice;

/// A map-like container that uses flatbuffer references.
/// Provides O(log n) lookup time using binary search on the sorted index.
pub(crate) struct FlatFilterMap<'a, I: PartialOrd + Copy, V> {
    index: &'a [I],
    values: Vector<'a, ForwardsUOffset<V>>,
}

/// Iterator over NetworkFilter objects from [FlatFilterMap]
pub(crate) struct FlatFilterMapIterator<'a, I: PartialOrd + Copy, V> {
    current_index: usize,
    key: I,
    indexes: &'a [I],
    values: Vector<'a, ForwardsUOffset<V>>,
}

fn partition_point<T, P>(index: Vector<T>, pred: P) -> usize
where
    P: FnMut(&T) -> bool,
{
    let s = fb_vector_to_slice(index);
    s.partition_point(pred)
}

impl<'a, I, V> Iterator for FlatFilterMapIterator<'a, I, V>
where
    I: PartialOrd + Copy,
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

impl<'a, I: PartialOrd + Copy, V> FlatFilterMap<'a, I, V> {
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

impl<I: PartialOrd + Copy, V> FlatFilterMap<'_, I, V> {
    #[cfg(test)]
    pub fn total_size(&self) -> usize {
        self.index.len()
    }
}

pub trait FlatSerialize<'a>: Sized {
    type Output: Sized + flatbuffers::Push + Clone;
    fn serialize(&self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output;
}

impl<'a> FlatSerialize<'a> for String {
    type Output = WIPOffset<&'a str>;
    fn serialize(&self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
        builder.create_string(self)
    }
}

impl<'a> FlatSerialize<'a> for u32 {
    type Output = u32;
    fn serialize(&self, _builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
        *self
    }
}

impl<'a, T> FlatSerialize<'a> for WIPOffset<T> {
    type Output = WIPOffset<T>;
    fn serialize(&self, _builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
        *self
    }
}

#[derive(Default)]
pub(crate) struct FlatFilterSetBuilder<I> {
    keys: HashSet<I>,
}

impl<'a, I: FlatSerialize<'a> + Ord + std::hash::Hash> FlatFilterSetBuilder<I> {
    pub fn insert(&mut self, key: I) {
        self.keys.insert(key);
    }

    pub fn finish(
        self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> WIPOffset<Vector<'a, <I::Output as flatbuffers::Push>::Output>> {
        let mut keys = self.keys.iter().collect::<Vec<_>>();
        keys.sort_unstable();
        let flat_keys = keys
            .into_iter()
            .map(|k| k.serialize(builder))
            .collect::<Vec<_>>();
        builder.create_vector(&flat_keys)
    }
}

pub(crate) struct FlatFilterSetView<'a, I> {
    keys: Vector<'a, I>,
}

impl<'a, I: Follow<'a>> FlatFilterSetView<'a, I>
where
    <I as Follow<'a>>::Inner: Ord,
{
    pub fn new(keys: Vector<'a, I>) -> Self {
        debug_assert!(keys.iter().is_sorted());
        Self { keys }
    }

    pub fn contains(&self, key: <I as Follow<'a>>::Inner) -> bool {
        self.keys.lookup_by_key(key, |a, b| a.cmp(b)).is_some()
    }
}

#[derive(Default)]
pub struct FlatMultiMapBuilder<I, V> {
    map: HashMap<I, Vec<V>>,
}

impl<'a, I: Eq + Ord + std::hash::Hash + FlatSerialize<'a>, V: FlatSerialize<'a>>
    FlatMultiMapBuilder<I, V>
{
    pub fn new_from_map(map: HashMap<I, Vec<V>>) -> Self {
        Self { map }
    }

    pub fn insert(&mut self, key: I, value: V) {
        self.map.entry(key).or_default().push(value);
    }

    pub fn finish(
        self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> (
        WIPOffset<Vector<'a, <I::Output as flatbuffers::Push>::Output>>,
        WIPOffset<Vector<'a, <V::Output as flatbuffers::Push>::Output>>,
    ) {
        let mut entries: Vec<_> = self.map.into_iter().collect();
        entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
        let mut indexes = Vec::with_capacity(entries.len());
        let mut values = Vec::with_capacity(entries.len());

        for (key, mv) in entries.iter() {
            let index = key.serialize(builder);
            for value in mv.iter() {
                indexes.push(index.clone());
                values.push(value.serialize(builder));
            }
        }

        let indexes = builder.create_vector(&indexes);
        let values = builder.create_vector(&values);

        (indexes, values)
    }

    // pub fn finish2(self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) ->
    //   (WIPOffset<Vector<'a, <I::Output as flatbuffers::Push>::Output>>, WIPOffset<Vector<'a, ForwardsUOffset<Vector<'a, <<V as FlatSerialize<'a>>::Output as flatbuffers::Push>::Output>>>>)
    // {
    //   let mut entries: Vec<_> = self.map.into_iter().collect();
    //   entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
    //   let mut indexes = Vec::with_capacity(entries.len());
    //   let mut values = Vec::with_capacity(entries.len());

    //   for (k, mv) in entries.into_iter() {
    //       let flat_mv = mv.into_iter().map(|v| v.serialize(builder)).collect::<Vec<_>>();
    //       indexes.push(k.serialize(builder));
    //       values.push(builder.create_vector(&flat_mv));
    //   }

    //   let indexes = builder.create_vector(&indexes);
    //   let values = builder.create_vector(&values);

    //   return (indexes, values);
    // }
}

pub(crate) struct FlatFilterMapView<'a, I: Ord + Copy + Follow<'a>, V> {
    index: Vector<'a, I>,
    values: Vector<'a, ForwardsUOffset<V>>,
}

/// Iterator over NetworkFilter objects from [FlatFilterMap]
/// TODO: use partition_point to find the right index
pub(crate) struct FlatFilterMapViewIterator<'a, 'b, I: Ord + Copy + Follow<'a>, V> {
    current_index: usize,
    key: &'b I::Inner,
    indexes: Vector<'a, I>,
    values: Vector<'a, ForwardsUOffset<V>>,
}

impl<'a, I, V> Iterator for FlatFilterMapViewIterator<'a, '_, I, V>
where
    I: Ord + Copy + Follow<'a>,
    V: Follow<'a>,
    I::Inner: Ord,
{
    type Item = (usize, <V as Follow<'a>>::Inner);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.indexes.len() {
            let p = self.indexes.get(self.current_index);
            if p.cmp(self.key) != Ordering::Equal {
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

impl<'a, I: Ord + Copy + Follow<'a>, V> FlatFilterMapView<'a, I, V>
where
    I::Inner: Ord,
{
    /// Construct [FlatFilterMap] from two vectors:
    /// - index: sorted array of keys
    /// - values: array of values, same length as index
    pub fn new(index: Vector<'a, I>, values: Vector<'a, ForwardsUOffset<V>>) -> Self {
        // Sanity check the size are equal. Note: next() will handle |values| correctly.
        debug_assert!(index.len() == values.len());

        // debug_assert!(index.iter().is_sorted());

        Self { index, values }
    }

    //   fn partition_point<P>(&self, mut pred: P) -> Option<usize>
    //   where
    //     P: FnMut(&I::Inner) -> bool {
    //     if self.index.is_empty() {
    //         return None;
    //     }

    //     let mut left: usize = 0;
    //     let mut right = self.index.len() - 1;

    //     while left <= right {
    //         let mid = (left + right) / 2;
    //         let value = self.index.get(mid);
    //         match pred(&value) {
    //             false => left = mid + 1,
    //             true => {
    //               if mid == 0 {
    //                 return None;
    //               }
    //               right = mid - 1;
    //             },
    //         }
    //     }

    //     None
    // }

    // /// Get an iterator over NetworkFilter objects with the given hash key.
    // pub fn get(&self, key: &I::Inner) -> FlatFilterMapViewIterator<'a, '_, I, V> {
    //     let start = partition_point(&self.index, |x: &I::Inner| x.cmp(key) == Ordering::Less);
    //     FlatFilterMapViewIterator {
    //         current_index: start,
    //         key,
    //         indexes: self.index,
    //         values: self.values,
    //     }
    // }
}
