#[allow(unknown_lints)]
#[allow(
    dead_code,
    clippy::all,
    unused_imports,
    unsafe_code,
    mismatched_lifetime_syntaxes
)]
#[path = "./test_containers_generated.rs"]
pub mod flat;
#[cfg(test)]
mod tests {
    use super::super::*;
    use super::flat::fb_test;

    fn serialize_set(values: Vec<&str>) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut set = HashSetBuilder::default();
        for value in values {
            set.insert(value.to_string());
        }
        let test_string_set = Some(FlatSerialize::serialize(set, &mut builder));

        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_string_set,
                ..Default::default()
            },
        );
        builder.finish(root, None);
        builder.finished_data().to_vec()
    }

    fn load_set<'a>(
        data: &'a [u8],
    ) -> HashSetView<&'a str, flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>> {
        let root = fb_test::root_as_test_root(data).unwrap();
        let flat_set = root.test_string_set().unwrap();
        HashSetView::new(flat_set)
    }

    #[test]
    fn test_empty_map() {
        let values = vec![];
        let data = serialize_set(values);
        let set = load_set(&data);
        assert_eq!(set.len(), 0);
        assert_eq!(set.capacity(), 4);
        assert!(!set.contains("a"));
    }

    #[test]
    fn test_duplicate_keys() {
        let values = vec!["b", "a", "b"];
        let data = serialize_set(values);
        let set = load_set(&data);
        assert_eq!(set.len(), 2);
        assert_eq!(set.capacity(), 4);
        assert!(set.contains("a"));
        assert!(set.contains("b"));
    }

    #[test]
    fn test_string_builder() {
        let values = vec!["b", "a", "c"];
        let data = serialize_set(values);
        let set = load_set(&data);

        assert!(set.contains("a"));
        assert!(set.contains("b"));
        assert!(set.contains("c"));
        assert!(!set.contains("d"));
        assert!(!set.contains(""));
    }
}
