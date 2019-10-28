extern crate adblock;

use adblock::request::Request;
use adblock::filters::network::NetworkFilter;
use adblock::filters::network::NetworkMatchable;
use adblock::engine::Engine;
use adblock::resources::{Resource, ResourceType, MimeType};

use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::prelude::*;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct TestRuleRequest {
    sourceUrl: String,
    url: String,
    r#type: String,
    filters: Vec<String>
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
    filters.iter()
        .map(|r| NetworkFilter::parse(&r, true))
        .filter_map(Result::ok)
        .filter(|f| f.is_redirect())
        .map(|f| {
            let redirect = f.redirect.unwrap();

            Resource {
                name: redirect.to_owned(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::from_extension(&redirect)),
                content: redirect,
            }
        })
        .collect()
}


#[test]
fn check_filter_matching() {
    let requests = load_requests();

    let mut requests_checked = 0;

    assert!(requests.len() > 0, "List of parsed request info is empty");

    for req in requests {
        for filter in req.filters {
            let nework_filter_res = NetworkFilter::parse(&filter, true);
            assert!(nework_filter_res.is_ok(), "Could not parse filter {}", filter);
            let network_filter = nework_filter_res.unwrap();

            let request_res = Request::from_urls(&req.url, &req.sourceUrl, &req.r#type);
            // The dataset has cases where URL is set to just "http://" or "https://", which we do not support
            if request_res.is_ok() {
                let request = request_res.unwrap();
                assert!(network_filter.matches(&request), "Expected {} to match {} at {}, typed {}", filter, req.url, req.sourceUrl, req.r#type);
                requests_checked += 1;
            }
        }
    }

    assert_eq!(requests_checked, 9381); // A catch for regressions
}

#[test]
fn check_engine_matching() {
    let requests = load_requests();

    assert!(requests.len() > 0, "List of parsed request info is empty");

    for req in requests {
        if req.url == "http://" || req.url == "https://" {
            continue;
        }
        for filter in req.filters {
            let mut engine = Engine::from_rules_debug(&[filter.clone()]);
            let resources = build_resources_from_filters(&[filter.clone()]);
            engine.with_resources(&resources);

            let nework_filter_res = NetworkFilter::parse(&filter, true);
            assert!(nework_filter_res.is_ok(), "Could not parse filter {}", filter);
            let network_filter = nework_filter_res.unwrap();

            let result = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);

            if network_filter.is_exception() {
                assert!(!result.matched, "Expected {} to NOT match {} at {}, typed {}", filter, req.url, req.sourceUrl, req.r#type);
                // assert!(result.exception.is_some(), "Expected exception {} to match {} at {}, typed {}", filter, req.url, req.sourceUrl, req.r#type);
            } else {
                assert!(result.matched, "Expected {} to match {} at {}, typed {}", filter, req.url, req.sourceUrl, req.r#type);
            }
            
            if network_filter.is_redirect() {
                assert!(result.redirect.is_some(), "Expected {} to trigger redirect rule {}", req.url, filter);
                let redirect = result.redirect.as_ref().unwrap();
                // each redirect url is base64 encoded
                assert!(redirect.contains("base64"));
            }
        }
    }
}
