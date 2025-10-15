/// An inner implementation of a HashMap-like container with open addressing.
/// Designed to be used in HashMap, HashSet, HashMultiMap.
/// The load factor is 25%-50%.
/// Uses RustC FxHasher as a hash function.
/// A default value is used to mark empty slots, so it can't be used as a key.
/// Inspired by https://source.chromium.org/chromium/chromium/src/+/main:components/url_pattern_index/closed_hash_map.h
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

pub fn find_slot<I: std::hash::Hash>(
    key: &I,
    table_size: usize,
    probe: impl Fn(usize) -> bool,
) -> usize {
    debug_assert!(table_size.is_power_of_two());
    let table_mask = table_size - 1;
    let mut slot = get_hash(&key) & table_mask;
    let mut step = 1;
    loop {
        if probe(slot) {
            return slot;
        }
        slot = (slot + step) & table_mask;
        step += 1;
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

    fn capacity(&self) -> usize {
        self.indexes.len()
    }

    pub fn get_single(&self, key: I) -> Option<V> {
        let slot = find_slot(&key, self.capacity(), |slot| -> bool {
            FbHashKey::is_empty(&self.indexes.get(slot)) || self.indexes.get(slot) == key
        });
        if FbHashKey::is_empty(&self.indexes.get(slot)) {
            None
        } else {
            Some(self.values.get(slot))
        }
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

        let slot = find_slot(&key, self.capacity(), |slot| -> bool {
            HashKey::is_empty(&self.indexes[slot])
                || (self.indexes[slot] == key && !allow_duplicates)
        });

        if HashKey::is_empty(&self.indexes[slot]) {
            self.indexes[slot] = key;
            self.values[slot] = value;
            self.size += 1;
            self.maybe_increase_capacity();
            (slot, &mut self.values[slot])
        } else {
            self.values[slot] = value;
            (slot, &mut self.values[slot])
        }
    }

    fn capacity(&self) -> usize {
        self.indexes.len()
    }

    pub fn get_or_insert(&mut self, key: I, value: V) -> &mut V {
        let slot = find_slot(&key, self.capacity(), |slot| -> bool {
            HashKey::is_empty(&self.indexes[slot]) || self.indexes[slot] == key
        });
        if !HashKey::is_empty(&self.indexes[slot]) {
            return &mut self.values[slot];
        }
        let (_, new_value) = self.insert(key, value, false);
        new_value
    }

    fn maybe_increase_capacity(&mut self) {
        if self.size * 2 <= self.capacity() { // Use 50% load factor.
            return;
        }

        let new_capacity = (self.capacity() * 2).next_power_of_two();
        let old_indexes = std::mem::take(&mut self.indexes);
        let old_values = std::mem::take(&mut self.values);
        self.indexes = vec![I::default(); new_capacity];
        self.values = vec![V::default(); new_capacity];

        for (key, value) in old_indexes.into_iter().zip(old_values.into_iter()) {
            if !HashKey::is_empty(&key) {
              let slot = find_slot(&key, new_capacity, |slot| -> bool {
                HashKey::is_empty(&self.indexes[slot])
              });
              self.indexes[slot] = key;
              self.values[slot] = value;
            }
        }
    }

    pub fn consume(value: Self) -> (Vec<I>, Vec<V>) {
        (value.indexes, value.values)
    }
}
