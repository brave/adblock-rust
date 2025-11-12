#[cfg(all(test, feature = "debug-info"))]
mod tests {
    use super::super::*;

    use crate::{request, Engine};

    use mock_instant::thread_local::MockClock;

    fn make_engine(line: &str) -> Engine {
        Engine::from_rules(vec![line], Default::default())
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
        let engine = make_engine("||geo*.hltv.org^");
        engine.borrow_regex_manager();

        assert!(
            engine
                .check_network_request(&make_request("https://geo2.hltv.org/"))
                .matched
        );

        let regex_manager = engine.borrow_regex_manager();
        assert_eq!(get_active_regex_count(&regex_manager), 1);
        assert_eq!(regex_manager.get_debug_regex_data().len(), 1);
    }

    #[test]
    fn discard_and_recreate() {
        let engine = make_engine("||geo*.hltv.org^");
        engine.borrow_regex_manager();

        assert!(
            engine
                .check_network_request(&make_request("https://geo2.hltv.org/"))
                .matched
        );

        {
            let regex_manager = engine.borrow_regex_manager();
            assert_eq!(regex_manager.get_compiled_regex_count(), 1);
            assert_eq!(get_active_regex_count(&regex_manager), 1);
        }

        {
            let regex_manager = engine.borrow_regex_manager();
            MockClock::advance(DEFAULT_DISCARD_UNUSED_TIME - Duration::from_secs(1));
            // The entry shouldn't be discarded because was used during
            // last REGEX_MANAGER_DISCARD_TIME.
            assert_eq!(get_active_regex_count(&regex_manager), 1);

            // The entry is entry is outdated, but should be discarded only
            // in the next cleanup() call. The call was 2 sec ago and is throttled
            // now.
            MockClock::advance(DEFAULT_CLEAN_UP_INTERVAL - Duration::from_secs(1));
            assert_eq!(get_active_regex_count(&regex_manager), 1);
        }

        {
            MockClock::advance(Duration::from_secs(2));
            let regex_manager = engine.borrow_regex_manager();
            // The entry is now outdated & cleanup() should be called => discard.
            assert_eq!(get_active_regex_count(&regex_manager), 0);
        }

        // The entry is recreated, get_compiled_regex_count() increased +1.
        assert!(
            engine
                .check_network_request(&make_request("https://geo2.hltv.org/"))
                .matched
        );
        let regex_manager = engine.borrow_regex_manager();
        assert_eq!(regex_manager.get_compiled_regex_count(), 2);
        assert_eq!(get_active_regex_count(&regex_manager), 1);
    }
}
