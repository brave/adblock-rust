use adblock::filters::network::{
    FilterPart, NetworkFilter, NetworkFilterMaskHelper, NetworkMatchable,
};
use adblock::regex_manager::RegexManager;
use adblock::request::Request;
use adblock::resources::{MimeType, Resource, ResourceType};
use adblock::Engine;

use base64::{engine::Engine as _, prelude::BASE64_STANDARD};
use serde::{Deserialize, Serialize};

use adblock::lists::ParseOptions;
use std::fs::File;
use std::io::prelude::*;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct TestRuleRequest {
    sourceUrl: String,
    url: String,
    r#type: String,
    filters: Vec<String>,
}

fn load_requests() -> Vec<TestRuleRequest> {
    let mut f = File::open("data/matching-test-requests.json").expect("file not found");

    let mut requests_str = String::new();
    f.read_to_string(&mut requests_str)
        .expect("something went wrong reading the file");

    let reqs: Vec<TestRuleRequest> = serde_json::from_str(&requests_str).unwrap();
    reqs
}

fn build_resources_from_filters(filters: &[String]) -> Vec<Resource> {
    filters
        .iter()
        .map(|r| NetworkFilter::parse(r, true, Default::default()))
        .filter_map(Result::ok)
        .filter(|f| f.is_redirect())
        .map(|f| {
            let redirect = f.modifier_option.unwrap();

            Resource {
                name: redirect.to_owned(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::from_extension(&redirect)),
                content: BASE64_STANDARD.encode(redirect),
                dependencies: vec![],
                permission: Default::default(),
            }
        })
        .collect()
}

#[test]
fn check_filter_matching() {
    let requests = load_requests();

    let mut requests_checked = 0;

    assert!(!requests.is_empty(), "List of parsed request info is empty");

    let opts = ParseOptions::default();

    for req in requests {
        for filter in req.filters {
            let network_filter_res = NetworkFilter::parse(&filter, true, opts);
            assert!(
                network_filter_res.is_ok(),
                "Could not parse filter {}",
                filter
            );
            let network_filter = network_filter_res.unwrap();

            let request_res = Request::new(&req.url, &req.sourceUrl, &req.r#type);
            // The dataset has cases where URL is set to just "http://" or "https://", which we do not support
            if request_res.is_ok() {
                let request = request_res.unwrap();
                assert!(
                    network_filter.matches(&request, &mut RegexManager::default()),
                    "Expected {} to match {} at {}, typed {}",
                    filter,
                    req.url,
                    req.sourceUrl,
                    req.r#type
                );
                requests_checked += 1;
            }
        }
    }

    assert_eq!(requests_checked, 9354); // A catch for regressions
}

#[test]
fn check_engine_matching() {
    let requests = load_requests();

    assert!(!requests.is_empty(), "List of parsed request info is empty");

    for req in requests {
        if req.url == "http://" || req.url == "https://" {
            continue;
        }
        for filter in req.filters {
            let opts = ParseOptions::default();
            let mut engine = Engine::from_rules_debug(&[filter.clone()], opts);
            let resources = build_resources_from_filters(&[filter.clone()]);
            engine.use_resources(resources);

            let network_filter_res = NetworkFilter::parse(&filter, true, opts);
            assert!(
                network_filter_res.is_ok(),
                "Could not parse filter {}",
                filter
            );
            let network_filter = network_filter_res.unwrap();

            let request = Request::new(&req.url, &req.sourceUrl, &req.r#type).unwrap();
            let result = engine.check_network_request(&request);

            if network_filter.is_exception() {
                assert!(
                    !result.matched,
                    "Expected {} to NOT match {} at {}, typed {}",
                    filter, req.url, req.sourceUrl, req.r#type
                );
                // assert!(result.exception.is_some(), "Expected exception {} to match {} at {}, typed {}", filter, req.url, req.sourceUrl, req.r#type);
            } else {
                assert!(
                    result.matched,
                    "Expected {} to match {} at {}, typed {}",
                    filter, req.url, req.sourceUrl, req.r#type
                );
            }

            if network_filter.is_redirect() {
                assert!(
                    result.redirect.is_some(),
                    "Expected {} to trigger redirect rule {}",
                    req.url,
                    filter
                );
                let resource = result.redirect.unwrap();
                // each redirect resource is base64 encoded
                assert!(resource.contains("base64"));
            }
        }
    }
}

#[test]
// #[cfg(not(debug_assertions))] // This test is too slow to run in debug mode
fn check_rule_matching_browserlike() {
    #[path = "../tests/test_utils.rs"]
    mod test_utils;
    use test_utils::rules_from_lists;

    use adblock::request::Request;
    use adblock::Engine;
    use serde::Deserialize;

    #[allow(non_snake_case)]
    #[derive(Deserialize)]
    struct TestRequest {
        frameUrl: String,
        url: String,
        cpt: String,
    }

    impl From<&TestRequest> for Request {
        fn from(v: &TestRequest) -> Self {
            Request::new(&v.url, &v.frameUrl, &v.cpt).unwrap()
        }
    }

    fn load_requests() -> Vec<TestRequest> {
        let requests_str = rules_from_lists(&["data/requests.json"]);
        requests_str
            .into_iter()
            .filter_map(|r| serde_json::from_str(&r).ok())
            .collect()
    }

    fn bench_rule_matching_browserlike(
        engine: &Engine,
        requests: &[TestRequest],
        rules: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> (u32, u32, f64, f64, f64, f64, f64, f64) {
        use adblock::filters::network::NetworkFilter;
        use adblock::lists::ParseOptions;

        let mut matches_count = 0u32;
        let mut passes_count = 0u32;

        for r in requests {
            let req: Request = r.into();
            if engine.check_network_request(&req).matched {
                matches_count += 1;
            } else {
                passes_count += 1;
            }
        }

        let mut has_tag_count = 0usize;
        let mut has_hostname_count = 0usize;
        let mut has_modifier_option_count = 0usize;
        let mut patterns_len_gt_one_count = 0usize;
        let mut opt_domains_gt_zero_count = 0usize;
        // let mut total_string_length = 0usize;
        let mut total_filters = 0f64;

        let opts = ParseOptions::default();
        for rule in rules {
            if let Ok(filter) = NetworkFilter::parse(rule.as_ref(), true, opts) {
                if filter.hostname.is_none() {
                    continue;
                }
                total_filters += 1.0;
                // total_string_length += rule.len();

                if filter.tag.is_some() {
                    has_tag_count += 1;
                }
                if filter.hostname.is_some() {
                    has_hostname_count += 1;
                }
                if filter.modifier_option.is_some() {
                    has_modifier_option_count += 1;
                }
                match filter.filter {
                    FilterPart::Empty => {}
                    FilterPart::Simple(s) => {
                      patterns_len_gt_one_count += 1;
                    }
                    FilterPart::AnyOf(s) => {
                        patterns_len_gt_one_count += 1;
                    }
                }

                if filter.opt_domains.unwrap_or_default().len()
                    + filter.opt_not_domains.unwrap_or_default().len()
                    > 0
                {
                    opt_domains_gt_zero_count += 1;
                }
            }
        }

        let tag_percentage = if total_filters > 0.0 {
            (has_tag_count as f64 / total_filters) * 100.0
        } else {
            0.0
        };
        let hostname_percentage = if total_filters > 0.0 {
            (has_hostname_count as f64 / total_filters) * 100.0
        } else {
            0.0
        };
        let modifier_option_percentage = if total_filters > 0.0 {
            (has_modifier_option_count as f64 / total_filters) * 100.0
        } else {
            0.0
        };
        let patterns_gt_one_percentage = if total_filters > 0.0 {
            (patterns_len_gt_one_count as f64 / total_filters) * 100.0
        } else {
            0.0
        };
        let opt_domains_gt_zero_percentage = if total_filters > 0.0 {
            (opt_domains_gt_zero_count as f64 / total_filters) * 100.0
        } else {
            0.0
        };

        println!("total_filters: {}", total_filters);

        (
            matches_count,
            passes_count,
            tag_percentage,
            hostname_percentage,
            modifier_option_percentage,
            patterns_gt_one_percentage,
            opt_domains_gt_zero_percentage,
            total_filters,
        )
    }

    let requests = load_requests();
    let rules = rules_from_lists(&["data/brave/brave-main-list.txt"]);
    let rules2 = rules_from_lists(&["data/brave/brave-main-list.txt"]);
    let engine = Engine::from_rules(rules, Default::default());
    let (
        blocked,
        passes,
        tag_pct,
        hostname_pct,
        modifier_opt_pct,
        patterns_pct,
        opt_domains_pct,
        total_filters,
    ) = bench_rule_matching_browserlike(&engine, &requests, rules2);
    println!("tag_pct: {}", tag_pct);
    println!("hostname_pct: {}", hostname_pct);
    println!("modifier_opt_pct: {}", modifier_opt_pct);
    println!("patterns_pct: {}", patterns_pct);
    println!("opt_domains_pct: {}", opt_domains_pct);
    println!("total_filters: {}", total_filters);

    let msg = "The number of blocked/passed requests has changed. ".to_string()
        + "If this is expected, update the expected values in the test.";
    assert_eq!((blocked, passes), (103973, 138972), "{}", msg);
}
