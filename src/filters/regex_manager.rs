//! A manager that creates/stores all regular expressions used by filters.
//! Rarely used entries could be discarded to save memory.
//! Non thread safe, the access must be synchronized externally.
use crate::filters::network::{compile_regex, CompiledRegex, NetworkFilter};
#[cfg(test)]
use mock_instant::{Instant, MockClock};
use std::collections::HashMap;
use std::time::Duration;

#[cfg(not(test))]
use std::time::Instant;

const REGEX_MANAGER_CLEAN_UP_INTERVAL: Duration = Duration::from_secs(30);
const REGEX_MANAGER_DISCARD_TIME: Duration = Duration::from_secs(180);

pub struct RegexDebugEntry {
    regex: Option<String>,
    last_used: Instant,
    usage_count: u64,
}

struct RegexEntry {
    regex: Option<CompiledRegex>,
    last_used: Instant,
    usage_count: u64,
}

type RandomState = std::hash::BuildHasherDefault<seahash::SeaHasher>;

pub struct RegexManager {
    map: HashMap<*const NetworkFilter, RegexEntry, RandomState>,
    compiled_regex_count: u64,
    now: Instant,
    last_cleanup: Instant,
}

impl Default for RegexManager {
    fn default() -> RegexManager {
        RegexManager {
            map: Default::default(),
            compiled_regex_count: 0,
            now: Instant::now(),
            last_cleanup: Instant::now(),
        }
    }
}

fn make_regexp(filter: &NetworkFilter) -> CompiledRegex {
    compile_regex(
        &filter.filter,
        filter.is_right_anchor(),
        filter.is_left_anchor(),
        filter.is_complete_regex(),
    )
}

impl RegexManager {
    pub fn matches(&mut self, filter: &NetworkFilter, pattern: &str) -> bool {
        if !filter.is_regex() && !filter.is_complete_regex() {
            return true;
        }
        let key = filter as *const NetworkFilter;
        use std::collections::hash_map::Entry;
        match self.map.entry(key) {
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                v.usage_count += 1;
                v.last_used = self.now;
                if v.regex.is_none() {
                    // A discarded entry, recreate it:
                    v.regex = Some(make_regexp(filter));
                    self.compiled_regex_count += 1;
                }
                return v.regex.as_ref().unwrap().is_match(pattern);
            }
            Entry::Vacant(e) => {
                self.compiled_regex_count += 1;
                let new_entry = RegexEntry {
                    regex: Some(make_regexp(filter)),
                    last_used: self.now,
                    usage_count: 1,
                };
                return e
                    .insert(new_entry)
                    .regex
                    .as_ref()
                    .unwrap()
                    .is_match(pattern);
            }
        };
    }

    pub fn update_time(&mut self) {
        self.now = Instant::now();
        if self.now - self.last_cleanup >= REGEX_MANAGER_CLEAN_UP_INTERVAL {
            self.last_cleanup = self.now;
            self.cleanup();
        }
    }

    pub fn cleanup(&mut self) {
        let now = self.now;
        for (_, v) in &mut self.map {
            if now - v.last_used >= REGEX_MANAGER_DISCARD_TIME {
                // Discard the regex to save memory.
                v.regex = None;
            }
        }
    }

    #[cfg(feature = "debug-info")]
    pub fn get_debug_regex_data(&self) -> Vec<RegexDebugEntry> {
        use itertools::Itertools;
        self.map
            .values()
            .map(|e| RegexDebugEntry {
                regex: e.regex.as_ref().map(|x| x.to_string()),
                last_used: e.last_used,
                usage_count: e.usage_count,
            })
            .collect_vec()
    }

    #[cfg(any(feature = "debug-info", test))]
    pub fn get_compiled_regex_count(&self) -> u64 {
        self.compiled_regex_count
    }
}

#[cfg(feature = "debug-info")]
mod tests {
    use super::*;
    #[cfg(test)]
    use crate::filters::network::NetworkMatchable;
    use crate::request;

    fn make_filter(line: &str) -> NetworkFilter {
        NetworkFilter::parse(line, true, Default::default()).unwrap()
    }

    fn make_request(url: &str) -> request::Request {
        request::Request::from_url(url).unwrap()
    }

    fn get_active_regex_count(regex_manager: &RegexManager) -> i32 {
        regex_manager
            .get_debug_regex_data()
            .iter()
            .fold(0, |acc, x| if x.regex.is_some() { acc + 1 } else { acc })
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

        MockClock::advance(REGEX_MANAGER_DISCARD_TIME - Duration::from_secs(1));
        regex_manager.update_time();
        // The entry shouldn't be discarded because was used during
        // last REGEX_MANAGER_DISCARD_TIME.
        assert_eq!(get_active_regex_count(&regex_manager), 1);

        // The entry is entry is outdated, but should be discarded only
        // in the next cleanup() call. The call was 2 sec ago and is throttled
        // now.
        MockClock::advance(REGEX_MANAGER_CLEAN_UP_INTERVAL - Duration::from_secs(1));
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
