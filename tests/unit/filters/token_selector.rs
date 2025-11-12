#[cfg(test)]
mod token_selector_tests {
    use super::super::*;
    use crate::utils::fast_hash;

    #[test]
    fn token_priority() {
        let selector = TokenSelector::new(0);
        let regular = fast_hash("rare_token");
        let worst = fast_hash("https");
        let bad = fast_hash("assets");

        assert_eq!(selector.select_least_used_token(&[]), 0);
        assert_eq!(selector.select_least_used_token(&[0, 0]), 0);

        // a regular token is always better
        assert_eq!(
            selector.select_least_used_token(&[regular, worst, bad]),
            regular
        );
        assert_eq!(
            selector.select_least_used_token(&[worst, bad, regular]),
            regular
        );

        // a bad token is always better than a worst token
        assert_eq!(selector.select_least_used_token(&[worst, bad]), bad);
        assert_eq!(selector.select_least_used_token(&[bad, worst]), bad);
    }

    #[test]
    fn test_select_least_used_token_with_usage() {
        let mut selector = TokenSelector::new(0);
        let token1 = fast_hash("token1");
        let token2 = fast_hash("token2");

        assert_eq!(selector.select_least_used_token(&[token1, token2]), token1);

        selector.record_usage(token1);
        selector.record_usage(token1);
        selector.record_usage(token2);

        // token2 should be selected as it has lower usage
        assert_eq!(selector.select_least_used_token(&[token1, token2]), token2);
    }
}
