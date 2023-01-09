use std::collections::HashMap;

use crate::filters::network::{compile_regex, CompiledRegex, NetworkFilter};

pub struct RegexDebugEntry {
    regex: String,
    last_used: std::time::Instant,
    usage_count: u64,
}

struct RegexEntry {
    regex: CompiledRegex,
    last_used: std::time::Instant,
    usage_count: u64,
}

type RandomState = std::hash::BuildHasherDefault<seahash::SeaHasher>;

pub struct RegexManager {
    map: HashMap<*const NetworkFilter, RegexEntry, RandomState>,
    compiled_regex_count: u64,
    now: std::time::Instant,
    last_cleanup: std::time::Instant,
}

impl Default for RegexManager {
    fn default() -> RegexManager {
        RegexManager {
            map: Default::default(),
            compiled_regex_count: 0,
            now: std::time::Instant::now(),
            last_cleanup: std::time::Instant::now(),
        }
    }
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
                return v.regex.is_match(pattern);
            }
            Entry::Vacant(e) => {
                let regex = compile_regex(
                    &filter.filter,
                    filter.is_right_anchor(),
                    filter.is_left_anchor(),
                    filter.is_complete_regex(),
                );
                self.compiled_regex_count += 1;
                let new_entry = RegexEntry {
                    regex,
                    last_used: self.now,
                    usage_count: 1,
                };
                return e.insert(new_entry).regex.is_match(pattern);
            }
        };
    }

    pub fn update_time(&mut self) {
        self.now = std::time::Instant::now();
        if self.now - self.last_cleanup > std::time::Duration::from_secs(30) {
            self.last_cleanup = self.now;
            self.cleanup();
        }
    }

    pub fn cleanup(&mut self) {
        let now = self.now;
        self.map.retain(|_, v| now - v.last_used < std::time::Duration::from_secs(180));
    }

    #[cfg(feature = "debug-info")]
    pub fn get_debug_regex_data(&self) -> Vec<RegexDebugEntry> {
        use itertools::Itertools;
        self.map.values().map(
            |e| RegexDebugEntry{regex: e.regex.to_string(),
                                            last_used : e.last_used,
                                            usage_count: e.usage_count})
            .collect_vec()
    }

    pub fn get_compiled_regex_count(&self) -> u64 {
        self.compiled_regex_count
    }

}
