extern crate adblock;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::request::Request;
use adblock::utils::rules_from_lists;

use serde::{Deserialize};
use std::fs::File;
use std::io::BufReader;

use csv;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct RequestRuleMatch {
    url: String,
    sourceUrl: String,
    r#type: String,
    blocked: u8,
    filter: Option<String>
}

fn load_requests() -> Vec<RequestRuleMatch> {
    let f = File::open("data/ublock-matches.tsv").expect("file not found");
    let reader = BufReader::new(f);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

    let mut reqs: Vec<RequestRuleMatch> = Vec::new();
    for result in rdr.deserialize() {
        if result.is_ok() {
            let record: RequestRuleMatch = result.unwrap();
            reqs.push(record);
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

fn get_blocker() -> Blocker {
  let rules = rules_from_lists(&vec![
    String::from("data/easylist.to/easylist/easylist.txt"),
    String::from("data/easylist.to/easylist/easyprivacy.txt")
  ]);

  let (network_filters, _) = adblock::lists::parse_filters(&rules, true, false, true);

  let blocker_options = BlockerOptions {
    debug: true,
    enable_optimizations: false,
    load_cosmetic_filters: false,
    load_network_filters: true
  };
  
  Blocker::new(network_filters, &blocker_options)
}

#[test]
fn check_specifics() {
    let blocker = get_blocker();

    {
        let request = adblock::request::Request::from_url("https://www.youtube.com/youtubei/v1/log_event?alt=json&key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8").unwrap();
        let checked = blocker.check(&request);
        assert_eq!(checked.matched, true);
    }
}

#[test]
fn check_basic_works_after_deserialization() {
    let blocker = get_blocker();
    let serialized = adblock::blocker::blocker_serialize(&blocker).unwrap();
    let deserialized_blocker = adblock::blocker::blocker_deserialize(&serialized).unwrap();

    {
        let request = adblock::request::Request::from_url("https://www.youtube.com/youtubei/v1/log_event?alt=json&key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8").unwrap();
        let checked = deserialized_blocker.check(&request);
        assert_eq!(checked.matched, true);
    }
}

#[test]
fn check_matching() {
    let requests = load_requests();

    assert!(requests.len() > 0, "List of parsed request info is empty");

    let blocker = get_blocker();

    let requests_len = requests.len() as u32;

    let mut mismatch_expected_match = 0;
    let mut mismatch_expected_exception = 0;
    let mut mismatch_expected_pass = 0;
    for req in requests {
        let request_res = Request::from_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert!(request_res.is_ok(), "Request couldn't be parsed: {} at {}, type {}", req.url, req.sourceUrl, req.r#type);
        let request = request_res.unwrap();

        let checked = blocker.check(&request);
        if req.blocked == 1 && checked.matched != true {
            mismatch_expected_match += 1;
            println!("Expected match, uBo matched {} at {}, type {} ON {:?}", req.url, req.sourceUrl, req.r#type, req.filter);
        } else if req.blocked == 2 && checked.exception.is_none() {
            mismatch_expected_exception += 1;
            println!("Expected exception to match for {} at {}, type {}, got rule match {:?}", req.url, req.sourceUrl, req.r#type, checked.filter);
        } else if req.blocked == 0 && checked.matched != false {
            mismatch_expected_pass += 1;
            println!("Expected pass, matched {} at {}, type {} ON {:?}", req.url, req.sourceUrl, req.r#type, checked.filter);
        }        
    }

    let mismatches = mismatch_expected_match + mismatch_expected_exception + mismatch_expected_pass;
    let ratio = mismatches as f32 / requests_len as f32;
    assert!(ratio < 0.05); 
}

#[test]
fn check_works_same_after_deserialization() {
    let requests = load_requests();

    assert!(requests.len() > 0, "List of parsed request info is empty");

    let blocker = get_blocker();
    let serialized = adblock::blocker::blocker_serialize(&blocker).unwrap();

    let deserialized_blocker = adblock::blocker::blocker_deserialize(&serialized).unwrap();

    for req in requests {
        let request_res = Request::from_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert!(request_res.is_ok(), "Request couldn't be parsed: {} at {}, type {}", req.url, req.sourceUrl, req.r#type);
        let request = request_res.unwrap();

        let checked = blocker.check(&request);
        let deserialized_checked = deserialized_blocker.check(&request);

        assert_eq!(checked.matched, deserialized_checked.matched);
        assert_eq!(checked.filter, deserialized_checked.filter);
        assert_eq!(checked.exception, deserialized_checked.exception);
        assert_eq!(checked.redirect, deserialized_checked.redirect);
    }

}

