#[cfg(test)]
mod tests {
    use super::super::*;

    /// Checks that for every probed key, a binary search over the directory range returns the same
    /// position as one over the whole array, and that the range holds every occurrence of the key.
    fn check_equivalent(keys: &[u32], probes: impl IntoIterator<Item = u32>) {
        let dir = PrefixDirectory::new(keys);
        for key in probes {
            let range = dir.range(key);
            assert!(range.start <= range.end && range.end <= keys.len());

            let start = keys.partition_point(|x| *x < key);
            let end = keys.partition_point(|x| *x <= key);
            let narrowed_start = range.start + keys[range.clone()].partition_point(|x| *x < key);
            assert_eq!(narrowed_start, start, "key {key:#x}");
            assert!(range.start <= start && end <= range.end, "key {key:#x}");
        }
    }

    fn edge_probes(keys: &[u32]) -> Vec<u32> {
        let mut probes = vec![0, 1, u32::MAX - 1, u32::MAX, 1 << 31, (1 << 31) - 1];
        for &k in keys {
            probes.extend([k, k.wrapping_sub(1), k.wrapping_add(1)]);
        }
        probes
    }

    /// A simple deterministic PRNG, to avoid adding a dependency.
    fn xorshift(state: &mut u64) -> u32 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        (*state >> 32) as u32
    }

    #[test]
    fn empty() {
        let dir = PrefixDirectory::new(&[]);
        assert_eq!(dir.range(0), 0..0);
        assert_eq!(dir.range(u32::MAX), 0..0);
    }

    #[test]
    fn small_arrays() {
        let keys = [0, 7, u32::MAX];
        let dir = PrefixDirectory::new(&keys);
        assert_eq!(dir.range(0), 0..keys.len());
        assert_eq!(dir.range(u32::MAX), 0..keys.len());
        check_equivalent(&keys, edge_probes(&keys));

        let keys = [0, 3, 3, 7, u32::MAX];
        let dir = PrefixDirectory::new(&keys);
        assert_eq!(dir.range(0), 0..4);
        assert_eq!(dir.range(u32::MAX), 4..5);
        check_equivalent(&keys, edge_probes(&keys));
    }

    #[test]
    fn extreme_keys() {
        let mut keys = vec![0; 40];
        keys.extend([u32::MAX; 40]);
        check_equivalent(&keys, edge_probes(&keys));
    }

    #[test]
    fn all_keys_equal() {
        for key in [0, 12345, u32::MAX] {
            let keys = vec![key; 1000];
            check_equivalent(&keys, edge_probes(&keys));
        }
    }

    #[test]
    fn random_keys_with_duplicates() {
        let mut state = 0x2545_f491_4f6c_dd1d;
        for len in [15, 16, 17, 100, 1000, 4096, 100_000] {
            let mut keys: Vec<u32> = (0..len).map(|_| xorshift(&mut state)).collect();
            // Duplicate some keys, as the network filter maps can contain several filters per token
            for i in (0..keys.len()).step_by(7) {
                keys.push(keys[i]);
            }
            keys.sort_unstable();
            let random_probes: Vec<u32> = (0..10_000).map(|_| xorshift(&mut state)).collect();
            check_equivalent(&keys, edge_probes(&keys).into_iter().chain(random_probes));
        }
    }

    #[test]
    fn clustered_keys() {
        // All keys share their top bits, so they land in a single bucket
        let keys: Vec<u32> = (0..10_000).map(|i| 0xabcd_0000 + i).collect();
        check_equivalent(&keys, edge_probes(&keys));
    }

    #[test]
    fn directory_size() {
        let keys: Vec<u32> = (0..100_000u32).map(|i| i.wrapping_mul(42_949)).collect();
        let mut keys = keys;
        keys.sort_unstable();
        let dir = PrefixDirectory::new(&keys);
        // One offset per bucket plus a sentinel; 2^15 buckets hold ~3 keys each
        assert_eq!(dir.offsets.len(), (1 << 15) + 1);

        let huge: Vec<u32> = (0..(1u32 << 18)).map(|i| i << 14).collect();
        let dir = PrefixDirectory::new(&huge);
        assert_eq!(dir.offsets.len(), (1 << MAX_PREFIX_BITS) + 1);
        check_equivalent(&huge, edge_probes(&huge[..1000]));
    }

    /// End-to-end check against the container it accelerates: `get_in` with a directory range
    /// must yield exactly the same values, in the same order, as an unrestricted `get`.
    #[test]
    fn multimap_get_in_matches_get() {
        use crate::flatbuffers::containers::flat_multimap::FlatMultiMapView;

        let mut state = 0x9e37_79b9_7f4a_7c15;
        let mut keys: Vec<u32> = (0..5000).map(|_| xorshift(&mut state)).collect();
        // Many values under some keys, including the extremes and a large "fallback" bucket at 0
        keys.extend([0; 60]);
        keys.extend([u32::MAX; 3]);
        for i in (0..5000).step_by(3) {
            keys.extend([keys[i]; 2]);
        }
        keys.sort_unstable();
        // Values are distinct, so equal sequences mean the same entries in the same order
        let values: Vec<u32> = (0..keys.len() as u32).collect();

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values_offset = builder.create_vector(&values);
        builder.finish(values_offset, None);
        let values =
            flatbuffers::root::<flatbuffers::Vector<u32>>(builder.finished_data()).unwrap();

        let map = FlatMultiMapView::new(keys.as_slice(), values);
        let dir = PrefixDirectory::new(&keys);
        let probes = edge_probes(&keys)
            .into_iter()
            .chain((0..2_000).map(|_| xorshift(&mut state)));
        for key in probes {
            let expected: Option<Vec<u32>> = map.get(key).map(|it| it.collect());
            let actual: Option<Vec<u32>> = map.get_in(key, dir.range(key)).map(|it| it.collect());
            assert_eq!(actual, expected, "key {key:#x}");
        }
    }
}
