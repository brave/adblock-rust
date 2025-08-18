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

    // Helper function to create a Vector from a slice
    fn create_vector_u32<'a>(
        builder: &'a mut flatbuffers::FlatBufferBuilder,
        data: &'a [u32],
    ) -> flatbuffers::Vector<'a, u32> {
        let vec_offset = builder.create_vector(data);
        builder.finish(vec_offset, None);
        let buf = builder.finished_data();
        flatbuffers::root::<flatbuffers::Vector<u32>>(buf).unwrap()
    }

    #[test]
    fn test_empty_map() {
        let index: &[u32] = &[];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[]);
        let map = FlatMultiMapView::new(index, values);

        assert_eq!(map.total_size(), 0);
        assert!(map.get(1).is_none());
    }

    #[test]
    fn test_single_element() {
        let index: &[u32] = &[1];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[100]);
        let map = FlatMultiMapView::new(index, values);

        assert_eq!(map.total_size(), 1);

        // Test existing key
        let mut iter = map.get(1).unwrap();
        assert_eq!(iter.next(), Some((0, 100)));
        assert_eq!(iter.next(), None);

        // Test non-existing key
        assert!(map.get(2).is_none());
    }

    #[test]
    fn test_multiple_elements() {
        let index: &[u32] = &[1, 1, 2, 2, 2, 3];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[10, 20, 30, 40, 50, 60]);

        let map = FlatMultiMapView::new(index, values);

        assert_eq!(map.total_size(), 6);

        // Test key with single value
        let mut iter = map.get(3).unwrap();
        assert_eq!(iter.next(), Some((5, 60)));
        assert_eq!(iter.next(), None);

        // Test key with multiple values
        let mut iter = map.get(2).unwrap();
        assert_eq!(iter.next(), Some((2, 30)));
        assert_eq!(iter.next(), Some((3, 40)));
        assert_eq!(iter.next(), Some((4, 50)));
        assert_eq!(iter.next(), None);

        // Test non-existing key
        assert!(map.get(4).is_none());
    }

    #[test]
    fn test_all_same_keys() {
        let index: &[u32] = &[5, 5, 5];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[100, 200, 300]);
        let map = FlatMultiMapView::new(index, values);

        assert_eq!(map.total_size(), 3);

        let mut iter = map.get(5).unwrap();
        assert_eq!(iter.next(), Some((0, 100)));
        assert_eq!(iter.next(), Some((1, 200)));
        assert_eq!(iter.next(), Some((2, 300)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_non_contiguous_keys() {
        let index: &[u32] = &[1, 3, 5];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[10, 30, 50]);
        let map = FlatMultiMapView::new(index, values);

        assert_eq!(map.total_size(), 3);

        assert_eq!(map.get(1).unwrap().next(), Some((0, 10)));
        assert_eq!(map.get(3).unwrap().next(), Some((1, 30)));
        assert_eq!(map.get(5).unwrap().next(), Some((2, 50)));
        assert!(map.get(2).is_none());
        assert!(map.get(4).is_none());
    }

    #[test]
    fn test_uint_builder() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut map = FlatMultiMapBuilder::<u64, u32>::default();
        map.insert(2, 20);
        map.insert(1, 10);
        map.insert(2, 30);
        let map = FlatMultiMapBuilder::finish(map, &mut builder);

        // Serialize to the test flatbuffer.
        let test_map = fb_test::TestUIntMap::create(
            &mut builder,
            &fb_test::TestUIntMapArgs {
                keys: Some(map.keys),
                values: Some(map.values),
            },
        );

        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_uint_map: Some(test_map),
                ..Default::default()
            },
        );
        builder.finish(root, None);

        // Load from the serialized test flatbuffer.
        use crate::flatbuffers::unsafe_tools::fb_vector_to_slice;
        let data = builder.finished_data();
        let root = fb_test::root_as_test_root(data).unwrap();
        let flat_map = root.test_uint_map().unwrap();
        let map = FlatMultiMapView::<u64, u32, &[u64]>::new(
            fb_vector_to_slice(flat_map.keys()),
            flat_map.values(),
        );

        assert_eq!(map.total_size(), 3);
        assert_eq!(map.get(1).unwrap().collect::<Vec<_>>(), vec![(0, 10)]);
        assert_eq!(
            map.get(2).unwrap().collect::<Vec<_>>(),
            vec![(1, 20), (2, 30)]
        );
        assert!(map.get(0).is_none());
        assert!(map.get(3).is_none());
    }

    #[test]
    fn test_string_builder() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut map = FlatMultiMapBuilder::<&str, &str>::default();
        map.insert("b", "20");
        map.insert("a", "10");
        map.insert("b", "30");
        let map = FlatMultiMapBuilder::finish(map, &mut builder);

        // Serialize to the test flatbuffer.
        let test_map = fb_test::TestStringMap::create(
            &mut builder,
            &fb_test::TestStringMapArgs {
                keys: Some(map.keys),
                values: Some(map.values),
            },
        );
        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_string_map: Some(test_map),
                ..Default::default()
            },
        );
        builder.finish(root, None);

        // Load from the serialized test flatbuffer.
        let data = builder.finished_data();
        let root = fb_test::root_as_test_root(data).unwrap();
        let flat_map = root.test_string_map().unwrap();
        let map = FlatMultiMapView::new(flat_map.keys(), flat_map.values());

        assert_eq!(map.total_size(), 3);
        assert_eq!(map.get("a").unwrap().collect::<Vec<_>>(), vec![(0, "10")]);
        assert_eq!(
            map.get("b").unwrap().collect::<Vec<_>>(),
            vec![(1, "20"), (2, "30")]
        );
        assert!(map.get("c").is_none());
        assert!(map.get("d").is_none());
    }
}
