use crate::filters::network::CompiledRegex;
use crate::filters::network::{NetworkFilter, NetworkFilterMask};
use crate::request::Request;
use itertools::*;
use std::collections::HashMap;
use regex::Regex;

trait Optimization {
    fn fusion(&self, filters: &Vec<NetworkFilter>) -> NetworkFilter;
    fn group_by_criteria(&self, filter: &NetworkFilter) -> String;
    fn select(&self, filter: &NetworkFilter) -> bool;
}

/**
 * Fusion a set of `filters` by applying optimizations sequentially.
 */
pub fn optimize(filters: Vec<NetworkFilter>) -> Vec<NetworkFilter> {
    let simple_pattern_group = SimplePatternGroup {};
    let filters_len = filters.len();
    let (mut fused, mut unfused) = apply_optimisation(&simple_pattern_group, filters);

    fused.append(&mut unfused);

    // println!("Optimized {} filters to {}", filters_len, fused.len());

    fused
}

fn apply_optimisation<T: Optimization>(
    optimization: &T,
    filters: Vec<NetworkFilter>,
) -> (Vec<NetworkFilter>, Vec<NetworkFilter>) {
    let (positive, mut negative): (Vec<NetworkFilter>, Vec<NetworkFilter>) =
        filters.into_iter().partition_map(|f| {
            if optimization.select(&f) {
                Either::Left(f)
            } else {
                Either::Right(f)
            }
        });

    let mut to_fuse: HashMap<String, Vec<NetworkFilter>> = HashMap::with_capacity(positive.len());
    positive
        .into_iter()
        .for_each(|f| insert_dup(&mut to_fuse, optimization.group_by_criteria(&f), f));

    let mut fused = Vec::with_capacity(to_fuse.len());
    for (_, group) in to_fuse {
        if group.len() > 1 {
            fused.push(optimization.fusion(&group));
        } else {
            group.into_iter().for_each(|f| negative.push(f));
        }
    }

    fused.shrink_to_fit();

    (fused, negative)
}

fn insert_dup<K, V>(map: &mut HashMap<K, Vec<V>>, k: K, v: V)
where
    K: std::cmp::Ord + std::hash::Hash,
{
    map.entry(k).or_insert_with(Vec::new).push(v)
}

#[derive(Debug, PartialEq)]
enum FusedPattern {
    MatchAll,
    MatchNothing,
    Pattern(String),
}

struct SimplePatternGroup {}

impl SimplePatternGroup {
    fn process_regex(r: CompiledRegex) -> FusedPattern {
        match r {
            CompiledRegex::MatchAll => FusedPattern::MatchAll,
            CompiledRegex::RegexParsingError(_e) => FusedPattern::MatchNothing,
            CompiledRegex::Compiled(r) => FusedPattern::Pattern(String::from(r.as_str())),
        }
    }

    fn escape(s: &String) -> String {
        lazy_static! {
            static ref ESCAPE_REF: Regex = Regex::new(r"([\+\?])").unwrap();
        }
        let repl_special = ESCAPE_REF.replace_all(&s, "\\$1");
        return format!("(?:{})", repl_special);
    }
}

impl Optimization for SimplePatternGroup {
    // Group simple patterns, into a single filter

    fn fusion(&self, filters: &Vec<NetworkFilter>) -> NetworkFilter {
        let patterns: Vec<_> = filters
            .iter()
            .map(|f| {
                if f.is_regex() {
                    SimplePatternGroup::process_regex(f.get_regex())
                } else if f.is_right_anchor() {
                    f.filter
                        .as_ref()
                        .map(|f| {
                            FusedPattern::Pattern(format!("{}$", SimplePatternGroup::escape(f)))
                        })
                        .unwrap_or_else(|| FusedPattern::MatchAll)
                } else if f.is_left_anchor() {
                    f.filter
                        .as_ref()
                        .map(|f| {
                            FusedPattern::Pattern(format!("^{}", SimplePatternGroup::escape(f)))
                        })
                        .unwrap_or_else(|| FusedPattern::MatchAll)
                } else {
                    f.filter
                        .as_ref()
                        .map(|f| FusedPattern::Pattern(SimplePatternGroup::escape(f)))
                        .unwrap_or_else(|| FusedPattern::MatchAll)
                }
            })
            .collect();

        let base_filter = filters.get(0).unwrap(); // FIXME: can technically panic, if filters list is empty
        let mut filter = base_filter.clone();
        // If there's anything in there that matches everything, whole regex matches everything
        if patterns.contains(&FusedPattern::MatchAll) {
            filter.filter = Some(String::from("")); // This will automatically compile to match-any
            filter.mask.set(NetworkFilterMask::IS_REGEX, true);
        } else {
            let joined_pattern = patterns
                .iter()
                .filter_map(|p| {
                    match p {
                        FusedPattern::MatchAll => None,     // should never get here
                        FusedPattern::MatchNothing => None, // just ignore
                        FusedPattern::Pattern(p) => Some(p),
                    }
                })
                .join("|");
            filter.filter = Some(format!("/{}/", joined_pattern));
            filter.mask.set(NetworkFilterMask::IS_REGEX, true);
            filter.mask.set(NetworkFilterMask::IS_COMPLETE_REGEX, true);
            if base_filter.raw_line.is_some() {
                filter.raw_line = Some(
                    filters
                        .iter()
                        .flat_map(|f| f.raw_line.clone())
                        .join(" <+> "),
                )
            }
        }

        filter
    }

    fn group_by_criteria(&self, filter: &NetworkFilter) -> String {
        filter.get_mask()
    }
    fn select(&self, filter: &NetworkFilter) -> bool {
        !filter.is_fuzzy()
            && filter.opt_domains.is_none()
            && filter.opt_not_domains.is_none()
            && !filter.is_hostname_anchor()
            && !filter.is_redirect()
            && !filter.is_csp()
            && !filter.has_bug()
    }
}

#[cfg(test)]
mod parse_tests {
    use super::*;
    use crate::lists;

    fn check_regex_match(regex: &CompiledRegex, pattern: &str, matches: bool) {
        let is_match = regex.is_match(pattern);
        assert!(is_match == matches, "Expected {} to match {}", regex.to_string(), pattern);
    }

    #[test]
    fn combines_simple_regex_patterns() {
        let rules = vec![
            String::from("/static/ad-"),
            String::from("/static/ad/*"),
            String::from("/static/ads/*"),
            String::from("/static/adv/*"),
        ];

        let (filters, _) = lists::parse_filters(&rules, true, false, true);

        let optimization = SimplePatternGroup {};

        filters
            .iter()
            .for_each(|f| assert!(optimization.select(f), "Expected rule to be selected"));

        let fused = optimization.fusion(&filters);

        assert!(fused.is_regex(), "Expected rule to be regex");
        assert_eq!(
            fused.to_string(),
            "/static/ad- <+> /static/ad/* <+> /static/ads/* <+> /static/adv/*"
        );

        let fused_regex = fused.get_regex();
        assert!(matches!(fused_regex, CompiledRegex::Compiled(_)));
        check_regex_match(&fused_regex, "/static/ad-", true);
        check_regex_match(&fused_regex, "/static/ads-", false);
        check_regex_match(&fused_regex, "/static/ad/", true);
        check_regex_match(&fused_regex, "/static/ad", false);
        check_regex_match(&fused_regex, "/static/ad/foobar", true);
        check_regex_match(&fused_regex, "/static/ad/foobar/asd?q=1", true);
        check_regex_match(&fused_regex, "/static/ads/", true);
        check_regex_match(&fused_regex, "/static/ads", false);
        check_regex_match(&fused_regex, "/static/ads/foobar", true);
        check_regex_match(&fused_regex, "/static/ads/foobar/asd?q=1", true);
        check_regex_match(&fused_regex, "/static/adv/", true);
        check_regex_match(&fused_regex, "/static/adv", false);
        check_regex_match(&fused_regex, "/static/adv/foobar", true);
        check_regex_match(&fused_regex, "/static/adv/foobar/asd?q=1", true);
    }

    #[test]
    #[ignore]
    fn separates_pattern_by_grouping() {
        let rules = vec![
            String::from("/analytics-v1."),
            String::from("/v1/pixel?"),
            String::from("/api/v1/stat?"),
            String::from("/analytics/v1/*$domain=~my.leadpages.net"),
            String::from("/v1/ads/*"),
        ];

        let (filters, _) = lists::parse_filters(&rules, true, false, true);

        let optimization = SimplePatternGroup {};

        let (fused, skipped) = apply_optimisation(&optimization, filters);

        assert_eq!(fused.len(), 1);
        let filter = fused.get(0).unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics-v1. <+> /v1/pixel? <+> /api/v1/stat? <+> /v1/ads/*"
        );

        assert!(filter.matches(&Request::from_urls("https://example.com/v1/pixel", "https://my.leadpages.net", "").unwrap()));

        assert_eq!(skipped.len(), 1);
        let filter = skipped.get(0).unwrap();
        assert_eq!(
            filter.to_string(),
            "/analytics/v1/*$domain=~my.leadpages.net"
        );

        assert!(filter.matches(&Request::from_urls("https://example.com/analytics/v1/foobar", "https://foo.leadpages.net", "").unwrap()))
    }

}
