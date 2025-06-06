#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    #[ignore] // won't match hard-coded values when using a different hash function
    fn fast_hash_matches_ts() {
        assert_eq!(fast_hash("hello world"), 4173747013); // cross-checked with the TS implementation
        assert_eq!(fast_hash("ello worl"), 2759317833); // cross-checked with the TS implementation
        assert_eq!(fast_hash(&"hello world"[1..10]), fast_hash("ello worl"));
        assert_eq!(fast_hash(&"hello world"[1..5]), fast_hash("ello"));
    }

    fn t(tokens: &[&str]) -> Vec<Hash> {
        tokens.iter().map(|t| fast_hash(t)).collect()
    }

    #[test]
    fn tokenize_filter_works() {
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&[]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, false).as_slice(),
            t(&[]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, true).as_slice(),
            t(&[]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, true).as_slice(),
            t(&[]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&[]).as_slice()
        );

        assert_eq!(
            tokenize_filter("foo/bar baz", false, false).as_slice(),
            t(&["foo", "bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, false).as_slice(),
            t(&["bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, true).as_slice(),
            t(&["bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", false, true).as_slice(),
            t(&["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo////bar baz", false, true).as_slice(),
            t(&["foo", "bar"]).as_slice()
        );
    }

    #[test]
    fn tokenize_works() {
        assert_eq!(tokenize("").as_slice(), t(&[]).as_slice());
        assert_eq!(tokenize("foo").as_slice(), t(&["foo"]).as_slice());
        assert_eq!(
            tokenize("foo/bar").as_slice(),
            t(&["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo-bar").as_slice(),
            t(&["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.bar").as_slice(),
            t(&["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.barƬ").as_slice(),
            t(&["foo", "barƬ"]).as_slice()
        );

        // Tokens cannot be surrounded by *
        assert_eq!(tokenize("foo.barƬ*").as_slice(), t(&["foo"]).as_slice());
        assert_eq!(tokenize("*foo.barƬ").as_slice(), t(&["barƬ"]).as_slice());
        assert_eq!(tokenize("*foo.barƬ*").as_slice(), t(&[]).as_slice());
    }

    #[test]
    fn bin_lookup_works() {
        assert!(!bin_lookup(&[], 42));
        assert!(bin_lookup(&[42], 42));
        assert!(bin_lookup(&[1, 2, 3, 4, 42], 42));
        assert!(bin_lookup(&[1, 2, 3, 4, 42], 1));
        assert!(bin_lookup(&[1, 2, 3, 4, 42], 3));
        assert!(!bin_lookup(&[1, 2, 3, 4, 42], 43));
        assert!(!bin_lookup(&[1, 2, 3, 4, 42], 0));
        assert!(!bin_lookup(&[1, 2, 3, 4, 42], 5));
    }
}
