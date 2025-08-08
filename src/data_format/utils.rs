//! Common utilities associated with serialization and deserialization of the `Engine` data into
//! binary formats.

use std::collections::{BTreeMap, HashMap};

use serde::{Serialize, Serializer};

/// Forces a `HashMap` to be serialized with a stable ordering by temporarily representing it as a
/// `BTreeMap`.
pub fn stabilize_hashmap_serialization<S, K, V>(
    set: &HashMap<K, V>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Ord + Serialize,
    V: Serialize,
{
    let stabilized: BTreeMap<&K, &V> = set.iter().collect();
    stabilized.serialize(s)
}
