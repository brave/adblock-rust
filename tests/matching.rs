extern crate adblock;

use adblock::request::Request;
use adblock::filters::network::NetworkFilter;

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


#[test]
fn check_matching() {
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


