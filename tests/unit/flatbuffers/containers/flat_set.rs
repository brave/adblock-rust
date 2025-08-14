#[allow(dead_code, clippy::all, unused_imports, unsafe_code)]
#[path = "./test_containers_generated.rs"]
pub mod flat;
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::*;
    use super::flat::fb_test;

    #[test]
    fn test_flat_set_view() {
        let data = vec![1, 2, 2, 3, 4, 4, 4, 5];
        let set = FlatSetView::<u32, &[u32]>::new(&data);

        // Test contains
        assert!(set.contains(1));
        assert!(set.contains(2));
        assert!(set.contains(4));
        assert!(!set.contains(6));

        // Test len
        assert_eq!(set.len(), 8);
    }

    #[test]
    fn test_uint_builder() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut set = HashSet::<u64>::default();
        set.insert(2);
        set.insert(1);
        set.insert(2);
        let set = FlatSerialize::serialize(set, &mut builder);

        // Serialize to the test flatbuffer.
        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_uint_set: Some(set),
                ..Default::default()
            },
        );
        builder.finish(root, None);

        // Load from the serialized test flatbuffer.
        use crate::flatbuffers::unsafe_tools::fb_vector_to_slice;
        let data = builder.finished_data();
        let root = fb_test::root_as_test_root(data).unwrap();
        let set =
            FlatSetView::<u64, &[u64]>::new(fb_vector_to_slice(root.test_uint_set().unwrap()));

        assert_eq!(set.len(), 2);
        assert!(set.contains(1));
        assert!(set.contains(2));
        assert!(!set.contains(3));
    }

    #[test]
    fn test_string_builder() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut set = HashSet::<&str>::default();
        set.insert("b");
        set.insert("a");
        set.insert("b");
        let set = FlatSerialize::serialize(set, &mut builder);

        // Serialize to the test flatbuffer.
        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_string_set: Some(set),
                ..Default::default()
            },
        );
        builder.finish(root, None);

        // Load from the serialized test flatbuffer.
        let data = builder.finished_data();
        let root = fb_test::root_as_test_root(data).unwrap();
        let set = FlatSetView::new(root.test_string_set().unwrap());

        assert_eq!(set.len(), 2);
        assert!(set.contains("a"));
        assert!(set.contains("b"));
        assert!(!set.contains("c"));
    }
}
