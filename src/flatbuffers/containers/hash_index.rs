/// An inner implementation of a HashMap-like container with open addressing.
/// Designed to be used in HashMap, HashSet, HashMultiMap.
/// The load factor is 25%-50%.
/// Uses RustC FxHasher as a hash function.
/// A default value is used to mark empty slots, so it can't be used as a key.
use std::marker::PhantomData;

use crate::flatbuffers::containers::fb_index::FbIndex;

pub(crate) trait HashKey: Eq + std::hash::Hash + Default + Clone {
    fn is_empty(&self) -> bool;
}

pub(crate) trait FbHashKey: Eq + std::hash::Hash {
    fn is_empty(&self) -> bool;
}

impl HashKey for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl FbHashKey for &str {
    fn is_empty(&self) -> bool {
        str::is_empty(self)
    }
}

#[inline(always)]
fn next_bucket(mut bucket: usize, capacity: usize, step: &mut usize) -> usize {
    bucket += *step * *step;
    *step += 1;
    bucket % capacity
}

fn find_matching_bucket<I: FbHashKey, Keys: FbIndex<I>>(
    indexes: &Keys,
    mut bucket: usize,
    key: I,
    capacity: usize,
    step: &mut usize,
) -> Option<usize> {
    debug_assert!(bucket < capacity);
    debug_assert!(*step > 0);
    debug_assert!(indexes.len() == capacity);
    loop {
        let data = indexes.get(bucket);
        if FbHashKey::is_empty(&data) {
            return None;
        }

        if data == key {
            return Some(bucket);
        }

        bucket = next_bucket(bucket, capacity, step);
    }
}

pub(crate) struct HashIndexView<I: FbHashKey, V, Keys: FbIndex<I>, Values: FbIndex<V>> {
    indexes: Keys,
    values: Values,
    _phantom_i: PhantomData<I>,
    _phantom_v: PhantomData<V>,
}

impl<I: FbHashKey, V, Keys: FbIndex<I>, Values: FbIndex<V>> HashIndexView<I, V, Keys, Values> {
    pub fn new(indexes: Keys, values: Values) -> Self {
        Self {
            indexes,
            values,
            _phantom_i: PhantomData,
            _phantom_v: PhantomData,
        }
    }

    pub fn get_single(&self, key: I) -> Option<V> {
        let bucket = self.find_single_bucket(key);
        bucket.map(|idx| self.values.get(idx))
    }

    fn find_single_bucket(&self, key: I) -> Option<usize> {
        let capacity = self.indexes.len();
        let bucket = get_hash(&key) % capacity;
        find_matching_bucket(&self.indexes, bucket, key, capacity, &mut 1)
    }
}

pub(crate) struct HashIndexBuilder<I, V> {
    indexes: Vec<I>,
    values: Vec<V>,
    size: usize,
}

fn get_hash<I: std::hash::Hash>(key: &I) -> usize {
    // RustC Hash is 2x faster than DefaultHasher.
    use rustc_hash::FxHasher;
    use std::hash::Hasher;
    let mut hasher = FxHasher::default();
    key.hash(&mut hasher);
    hasher.finish() as usize
}

impl<I: HashKey, V: Default + Clone> Default for HashIndexBuilder<I, V> {
    fn default() -> Self {
        Self::new_with_capacity(4)
    }
}

impl<I: HashKey, V: Default + Clone> HashIndexBuilder<I, V> {
    pub fn new_with_capacity(capacity: usize) -> Self {
        debug_assert!(capacity >= 4);
        let self_ = Self {
            size: 0,
            indexes: vec![I::default(); capacity],
            values: vec![V::default(); capacity],
        };
        debug_assert_eq!(self_.indexes.len(), capacity);
        debug_assert_eq!(self_.capacity(), capacity);
        self_
    }

    pub fn insert(&mut self, key: I, value: V, allow_duplicates: bool) -> (usize, &mut V) {
        debug_assert!(!HashKey::is_empty(&key), "Key is empty");
        let target_hash = get_hash(&key);

        let capacity = self.capacity();
        assert!(capacity >= 4);
        let mut bucket = target_hash % capacity;

        let mut step = 1;

        loop {
            if HashKey::is_empty(&self.indexes[bucket]) {
                // Found an empty bucket, take it and insert new key-value pair.
                self.indexes[bucket] = key;
                self.values[bucket] = value;
                self.size += 1;
                self.maybe_increase_capacity(allow_duplicates);
                return (bucket, &mut self.values[bucket]);
            }

            if self.indexes[bucket] == key && !allow_duplicates {
                // Update the value for an existing key.
                self.values[bucket] = value;
                return (bucket, &mut self.values[bucket]);
            }

            bucket = next_bucket(bucket, capacity, &mut step);
        }
    }

    fn capacity(&self) -> usize {
        self.indexes.len()
    }

    pub fn find_single_bucket(&mut self, key: &I) -> Option<usize> {
        let capacity = self.indexes.len();
        let mut bucket = get_hash(key) % capacity;
        let mut step = 1;
        loop {
            let data = &self.indexes[bucket];
            if HashKey::is_empty(data) {
                return None;
            }

            if data == key {
                return Some(bucket);
            }

            bucket = next_bucket(bucket, capacity, &mut step);
        }
    }

    pub fn get_or_insert(&mut self, key: I, value: V) -> &mut V {
        if let Some(existing_bucket) = self.find_single_bucket(&key) {
            return &mut self.values[existing_bucket];
        }
        let (_, new_value) = self.insert(key, value, false);
        new_value
    }

    fn maybe_increase_capacity(&mut self, allow_duplicates: bool) {
        // Use 50% load factor.
        if self.size * 2 > self.capacity() {
            self.size = 0;
            let new_capacity = self.capacity() * 2;
            let old_indexes = std::mem::take(&mut self.indexes);
            let old_values = std::mem::take(&mut self.values);
            self.indexes = vec![I::default(); new_capacity];
            self.values = vec![V::default(); new_capacity];

            for (key, value) in old_indexes.into_iter().zip(old_values.into_iter()) {
                if !HashKey::is_empty(&key) {
                    self.insert(key, value, allow_duplicates);
                }
            }
        }
    }

    pub fn consume(value: Self) -> (Vec<I>, Vec<V>) {
        (value.indexes, value.values)
    }
}
