#[cfg(test)]
mod tests {
    use super::super::*;

    // Helper function to create a Vector from a slice
    fn create_vector_u32<'a>(
        builder: &'a mut flatbuffers::FlatBufferBuilder,
        data: &'a [u32],
    ) -> flatbuffers::Vector<'a, u32> {
        let vec_offset = builder.create_vector(data);
        builder.finish(vec_offset, None);
        let buf = builder.finished_data();
        flatbuffers::root::<flatbuffers::Vector<u32>>(buf).expect("OK")
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
}
