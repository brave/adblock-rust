//! Optional instrumentation for network filter matching.
//!
//! Enable with the `match-debug-stats` feature, then call [`reset`] before a request
//! check and [`snapshot`] / [`take`] afterwards.

use std::cell::RefCell;

/// Which sequential check stage accepted or rejected a filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchStage {
    Options,
    IncludedDomains,
    ExcludedDomains,
    IncludedToDomains,
    ExcludedToDomains,
    Pattern,
}

/// Counters collected while matching network filters.
#[derive(Clone, Debug, Default)]
pub struct MatchDebugStats {
    /// Number of filters for which `matches()` was invoked.
    pub filters_checked: u64,
    /// Number of filters that returned a full match.
    pub filters_matched: u64,
    /// Stage of the last successful match (always [`MatchStage::Pattern`] today).
    pub last_match_stage: Option<MatchStage>,
    /// Stage that rejected the most recently checked non-matching filter.
    pub last_reject_stage: Option<MatchStage>,
    pub reject_options: u64,
    pub reject_included_domains: u64,
    pub reject_excluded_domains: u64,
    pub reject_included_to_domains: u64,
    pub reject_excluded_to_domains: u64,
    pub reject_pattern: u64,
}

thread_local! {
    static STATS: RefCell<MatchDebugStats> = RefCell::new(MatchDebugStats::default());
}

/// Clears collected match statistics.
pub fn reset() {
    STATS.with(|s| *s.borrow_mut() = MatchDebugStats::default());
}

/// Returns a copy of the current statistics.
pub fn snapshot() -> MatchDebugStats {
    STATS.with(|s| s.borrow().clone())
}

/// Returns the current statistics and resets the collector.
pub fn take() -> MatchDebugStats {
    STATS.with(|s| s.replace(MatchDebugStats::default()))
}

#[inline]
pub(crate) fn record_checked() {
    STATS.with(|s| s.borrow_mut().filters_checked += 1);
}

#[inline]
pub(crate) fn record_reject(stage: MatchStage) {
    STATS.with(|s| {
        let mut stats = s.borrow_mut();
        stats.last_reject_stage = Some(stage);
        match stage {
            MatchStage::Options => stats.reject_options += 1,
            MatchStage::IncludedDomains => stats.reject_included_domains += 1,
            MatchStage::ExcludedDomains => stats.reject_excluded_domains += 1,
            MatchStage::IncludedToDomains => stats.reject_included_to_domains += 1,
            MatchStage::ExcludedToDomains => stats.reject_excluded_to_domains += 1,
            MatchStage::Pattern => stats.reject_pattern += 1,
        }
    });
}

#[inline]
pub(crate) fn record_match(stage: MatchStage) {
    STATS.with(|s| {
        let mut stats = s.borrow_mut();
        stats.filters_matched += 1;
        stats.last_match_stage = Some(stage);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::Request;
    use crate::Engine;

    #[test]
    fn collects_filter_check_and_match_stage() {
        reset();
        let engine = Engine::new_with_list_text("||example.com^$script");
        let request =
            Request::new("https://example.com/ads.js", "https://foo.com/", "script", "").unwrap();
        assert!(engine.check_network_request(&request).should_block());
        let stats = take();
        assert!(stats.filters_checked >= 1);
        assert_eq!(stats.filters_matched, 1);
        assert_eq!(stats.last_match_stage, Some(MatchStage::Pattern));
    }
}
