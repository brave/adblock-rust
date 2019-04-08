extern crate adblock;

use adblock::utils;
use adblock::lists::parse_filters;
use adblock::filters::network::NetworkFilter;

#[cfg(feature="full-domain-matching")]
use itertools::Itertools;

fn get_network_filters() -> Vec<NetworkFilter> {
    let rules_lists = utils::rules_from_lists(&vec![
        String::from("data/easylist.to/easylist/easylist.txt"),
        String::from("data/easylist.to/easylist/easyprivacy.txt"),
    ]);

    let (network_filters, _) = parse_filters(&rules_lists, true, false, true);
    network_filters
}

#[cfg(feature="full-domain-matching")]
fn has_unique_elements<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + std::hash::Hash,
{
    let mut uniq = std::collections::HashSet::new();
    iter.into_iter().all(move |x| uniq.insert(x))
}

#[test]
fn check_rule_ids_no_collisions() {
    let network_filters = get_network_filters();
    let mut filter_ids: std::collections::HashMap<utils::Hash, String> = std::collections::HashMap::new();

    for filter in network_filters {
        let id = filter.get_id();
        let rule = filter.raw_line.unwrap_or_default();
        let existing_rule = filter_ids.get(&id);
        assert!(existing_rule.is_none() || existing_rule.unwrap() == &rule, "ID {} for {} already present from {}", id, rule, existing_rule.unwrap());
        filter_ids.insert(id, rule);
    }
}

#[cfg(feature="full-domain-matching")]
#[test]
fn check_domains_no_hash_collisions() {
    let network_filters = get_network_filters();

    for filter in network_filters {
        let rule = filter.raw_line.unwrap_or_default();
        if filter.opt_domains_full.is_some() {
            let mut domains: Vec<String> = filter.opt_domains_full.unwrap();
            domains.sort();
            domains.dedup();
            let hashes: Vec<adblock::utils::Hash> = domains.iter().map(|d: &String| utils::fast_hash(d)).collect();
            assert!(has_unique_elements(&hashes), "Collisions in {} among domains: {}", rule, domains.iter().join(" "))
        }
        if filter.opt_not_domains_full.is_some() {
            let mut domains: Vec<String> = filter.opt_not_domains_full.unwrap();
            domains.sort();
            domains.dedup();
            let hashes: Vec<adblock::utils::Hash> = domains.iter().map(|d: &String| utils::fast_hash(d)).collect();
            assert!(has_unique_elements(&hashes), "Collisions in {} among NOT domains: {}", rule, domains.iter().join(" "))
        }
    }
}