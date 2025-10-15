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

    fn serialize_map(values: Vec<(&str, &str)>) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let mut map = HashMapBuilder::default();
        for (key, value) in values {
            map.insert(key.to_string(), value.to_string());
        }
        let map = HashMapBuilder::finish(map, &mut builder);
        let map_serialized = fb_test::TestStringMap::create(
            &mut builder,
            &fb_test::TestStringMapArgs {
                keys: Some(map.keys),
                values: Some(map.values),
            },
        );
        let root = fb_test::TestRoot::create(
            &mut builder,
            &fb_test::TestRootArgs {
                test_string_map: Some(map_serialized),
                ..Default::default()
            },
        );
        builder.finish(root, None);
        builder.finished_data().to_vec()
    }

    fn load_map<'a>(data: &'a [u8]) -> HashMapStringView<'a, &'a str> {
        let root = fb_test::root_as_test_root(data).unwrap();
        let flat_map = root.test_string_map().unwrap();
        HashMapView::new(flat_map.keys(), flat_map.values())
    }

    #[test]
    fn test_empty_map() {
        let values = vec![];
        let data = serialize_map(values);
        let map = load_map(&data);
        assert_eq!(map.len(), 0);
        assert_eq!(map.capacity(), 4);
        assert!(map.get("a").is_none());
    }

    #[test]
    fn test_duplicate_keys() {
        let values = vec![("b", "20"), ("a", "10"), ("b", "30")];
        let data = serialize_map(values);
        let map = load_map(&data);
        assert_eq!(map.len(), 2);
        assert_eq!(map.capacity(), 4);
        assert_eq!(map.get("a").unwrap(), "10");
        assert_eq!(map.get("b").unwrap(), "30");
    }

    // TODO: test get_or_insert

    #[test]
    fn test_string_builder() {
        let values = vec![("b", "20"), ("a", "10"), ("c", "30")];
        let data = serialize_map(values);
        let map = load_map(&data);

        assert_eq!(map.get("a").unwrap(), "10");
        assert_eq!(map.get("b").unwrap(), "20");
        assert_eq!(map.get("c").unwrap(), "30");
        assert!(map.get("d").is_none());
        assert!(map.get("").is_none());
    }
}
