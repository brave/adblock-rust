#[cfg(test)]
mod tests {
    use super::super::*;

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
}
