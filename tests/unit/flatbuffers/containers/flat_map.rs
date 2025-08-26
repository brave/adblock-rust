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
    use std::collections::HashMap;

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
        let map = FlatMapView::new(index, values);

        assert_eq!(map.len(), 0);
        assert!(map.get(1).is_none());
    }

    #[test]
    fn test_multiple_elements() {
        let index: &[u32] = &[1, 2, 4, 6, 100, 102];
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let values = create_vector_u32(&mut builder, &[10, 20, 30, 40, 50, 60]);

        let map = FlatMapView::new(index, values);

        assert_eq!(map.len(), 6);

        assert_eq!(map.get(2), Some(20));
        assert_eq!(map.get(4), Some(30));
        assert_eq!(map.get(100), Some(50));
        assert_eq!(map.get(102), Some(60));
        assert!(map.get(103).is_none());
    }

    #[test]
    fn test_string_builder() {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut map = HashMap::new();
        map.insert("b", "20");
        map.insert("a", "10");
        map.insert("c", "30");
        let map = FlatMapBuilder::finish(map, &mut builder);

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
        let map = FlatMapView::new(flat_map.keys(), flat_map.values());

        assert_eq!(map.get("a").unwrap(), "10");
        assert_eq!(map.get("b").unwrap(), "20");
        assert_eq!(map.get("c").unwrap(), "30");
        assert!(map.get("d").is_none());
        assert!(map.get("").is_none());
    }
}
