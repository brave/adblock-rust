//! Holds the implementation of [FlatMapView].

use crate::filters::unsafe_tools::fb_vector_to_slice;
use flatbuffers::{Follow, ForwardsUOffset, Vector, WIPOffset};
use std::collections::{HashMap, HashSet};

fn sort_and_store_keys<'a, I: FlatSerialize<'a> + Ord + std::hash::Hash>(
    keys: impl Iterator<Item = I>,
    builder: &mut flatbuffers::FlatBufferBuilder<'a>,
) -> WIPOffset<Vector<'a, <I::Output as flatbuffers::Push>::Output>> {
    let mut keys = keys.collect::<Vec<_>>();
    keys.sort_unstable();
    // TODO: serialize in place, without collect::Vec<_>
    let flat_keys = keys
        .into_iter()
        .map(|k| k.serialize(builder))
        .collect::<Vec<_>>();
    builder.create_vector(&flat_keys)
}

// fn find_equal_range_pod_type<'a, I: Ord + Follow<'a>>(keys: &'a Vector<'a, I>, key: I) -> (usize, usize) {
//   let slice = fb_vector_to_slice(keys);
//   let start = slice.partition_point(|x| *x < key);
//   let end = slice.partition_point(|x| *x <= key);
//   (start, end)
// }

fn partition_point<'a, I: Follow<'a>>(
    keys: &'a Vector<'a, I>,
    pred: impl Fn(&I::Inner) -> bool,
) -> usize
where
    I::Inner: Ord,
{
    let mut start = 0;
    let mut end = keys.len();
    while start < end {
        let mid = (start + end) / 2;
        let mid_key = keys.get(mid);
        if pred(&mid_key) {
            start = mid + 1;
        } else {
            end = mid;
        }
    }
    end
}

fn find_equal_range<'a, I: Follow<'a>>(keys: &'a Vector<'a, I>, key: I::Inner) -> (usize, usize)
where
    I::Inner: Ord,
{
    let start = partition_point(keys, |x| *x < key);
    let end = partition_point(keys, |x| *x <= key);
    (start, end)
}

// Iterator over items in FlatMapView
pub(crate) struct FlatMapViewIterator<'a, V: Follow<'a>> {
    values: Vector<'a, ForwardsUOffset<V>>,
    current_idx: usize,
    end_idx: usize,
}

impl<'a, V: Follow<'a>> Iterator for FlatMapViewIterator<'a, V> {
    type Item = (usize, V::Inner);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx < self.end_idx {
            let idx = self.current_idx;
            let value = self.values.get(self.current_idx);
            self.current_idx += 1;
            Some((idx, value))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.end_idx - self.current_idx,
            Some(self.end_idx - self.current_idx),
        )
    }
}

pub(crate) struct FlatMapView<'a, I: Follow<'a>, V: Follow<'a>>
where
    I::Inner: Ord,
{
    index: Vector<'a, I>,
    values: Vector<'a, ForwardsUOffset<V>>,
}

impl<'a, I: Follow<'a>, V: Follow<'a>> FlatMapView<'a, I, V>
where
    I::Inner: Ord,
{
    pub fn new(index: Vector<'a, I>, values: Vector<'a, ForwardsUOffset<V>>) -> Self {
        debug_assert!(index.len() == values.len());
        debug_assert!(index.iter().is_sorted());
        Self { index, values }
    }

    pub fn get(&'a self, key: I::Inner) -> FlatMapViewIterator<'a, V> {
        let (start, end) = find_equal_range(&self.index, key);
        FlatMapViewIterator {
            values: self.values,
            current_idx: start,
            end_idx: end,
        }
    }
}

impl<'a, I: Follow<'a>, V: Follow<'a>> FlatMapView<'a, I, V>
where
    I::Inner: Ord,
{
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
        sort_and_store_keys(self.keys.into_iter(), builder)
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

impl<'a, I: Ord + std::hash::Hash + FlatSerialize<'a>, V: FlatSerialize<'a>>
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
}
