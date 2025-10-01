#[cfg(all(test, feature = "debug-info"))]
mod tests {
    use super::super::*;

    use crate::filters::network::{NetworkFilter, NetworkMatchable};
    use crate::request;

    use mock_instant::global::MockClock;

    fn make_filter(line: &str) -> NetworkFilter {
        NetworkFilter::parse(line, true, Default::default()).unwrap()
    }

    fn make_request(url: &str) -> request::Request {
        request::Request::new(url, "https://example.com", "other").unwrap()
    }

    fn get_active_regex_count(regex_manager: &RegexManager) -> usize {
        regex_manager
            .get_debug_regex_data()
            .iter()
            .filter(|x| x.regex.is_some())
            .count()
    }

    #[test]
    fn simple_match() {
        let mut regex_manager = RegexManager::default();
        regex_manager.update_time();

        let filter = make_filter("||geo*.hltv.org^");
        assert!(filter.matches(&make_request("https://geo2.hltv.org/"), &mut regex_manager));
        assert_eq!(get_active_regex_count(&regex_manager), 1);
        assert_eq!(regex_manager.get_debug_regex_data().len(), 1);
    }

    #[test]
    fn discard_and_recreate() {
        let mut regex_manager = RegexManager::default();
        regex_manager.update_time();

        let filter = make_filter("||geo*.hltv.org^");
        assert!(filter.matches(&make_request("https://geo2.hltv.org/"), &mut regex_manager));
        assert_eq!(regex_manager.get_compiled_regex_count(), 1);
        assert_eq!(get_active_regex_count(&regex_manager), 1);

        MockClock::advance(DEFAULT_DISCARD_UNUSED_TIME - Duration::from_secs(1));
        regex_manager.update_time();
        // The entry shouldn't be discarded because was used during
        // last REGEX_MANAGER_DISCARD_TIME.
        assert_eq!(get_active_regex_count(&regex_manager), 1);

        // The entry is entry is outdated, but should be discarded only
        // in the next cleanup() call. The call was 2 sec ago and is throttled
        // now.
        MockClock::advance(DEFAULT_CLEAN_UP_INTERVAL - Duration::from_secs(1));
        regex_manager.update_time();
        assert_eq!(get_active_regex_count(&regex_manager), 1);

        MockClock::advance(Duration::from_secs(2));
        regex_manager.update_time();
        // The entry is now outdated & cleanup() should be called => discard.
        assert_eq!(get_active_regex_count(&regex_manager), 0);

        // The entry is recreated, get_compiled_regex_count() increased +1.
        assert!(filter.matches(&make_request("https://geo2.hltv.org/"), &mut regex_manager));
        assert_eq!(regex_manager.get_compiled_regex_count(), 2);
        assert_eq!(get_active_regex_count(&regex_manager), 1);
    }
}
