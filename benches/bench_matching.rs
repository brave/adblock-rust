extern crate criterion;

use criterion::*;

use serde::{Deserialize, Serialize};
use serde_json;

use adblock;
use adblock::utils::{read_rules, rules_from_lists};
use adblock::blocker::{Blocker, BlockerOptions};
use adblock::request::Request;


fn default_lists() -> Vec<String> {
  rules_from_lists(vec![
    "data/easylist.to/easylist/easylist.txt",
  ])
}

#[derive(Serialize, Deserialize)]
struct TestRequest {
    frameUrl: String,
    url: String,
    cpt: String
}

fn load_requests() -> Vec<TestRequest> {
    let requests_str = adblock::utils::read_rules("data/requests.json");
    let reqs: Vec<TestRequest> = requests_str.into_iter().map(|r| serde_json::from_str(&r)).filter_map(Result::ok).collect();
    reqs
}

fn get_blocker(rules: &Vec<String>) -> Blocker {
  let (network_filters, _) = adblock::lists::parse_filters(rules, true, false, false);

  let blocker_options = BlockerOptions {
    debug: false,
    enable_optimizations: false,
    load_cosmetic_filters: false,
    load_network_filters: true
  };
  
  Blocker::new(network_filters, &blocker_options)
}

fn bench_rule_matching(blocker: &Blocker, requests: &Vec<TestRequest>) -> (u32, u32, u32) {
  let mut matches = 0;
  let mut passes = 0;
  let mut errors = 0;
  requests
    .iter()
    .for_each(|r| {
      let req: Result<Request, _> = Request::from_urls(&r.url, &r.frameUrl, &r.cpt);
      match req.map(|parsed| blocker.check(&parsed)).as_ref() {
        Ok(check) if check.matched => matches += 1,
        Ok(_) => passes += 1,
        Err(_) => errors += 1
      }
    });
  println!("Got {} matches, {} passes, {} errors", matches, passes, errors);  
  (matches, passes, errors)
}

fn bench_matching_only(blocker: &Blocker, requests: &Vec<Request>) -> (u32, u32) {
  let mut matches = 0;
  let mut passes = 0;
  requests
    .iter()
    .for_each(|parsed| {
      let check =  blocker.check(&parsed);
      if check.matched {
        matches += 1;
      } else {
        passes += 1;
      }
    });
  println!("Got {} matches, {} passes", matches, passes);  
  (matches, passes)
}

fn rule_match(c: &mut Criterion) {
  
  let rules = default_lists();
  let requests = load_requests();
  let requests_len = requests.len() as u32;
  c.bench(
        "rule-match",
        Benchmark::new(
            "easylist",
            move |b| {
              let blocker = get_blocker(&rules);
              b.iter(|| bench_rule_matching(&blocker, &requests))
            },
        ).throughput(Throughput::Elements(requests_len))
        .sample_size(10)
    );
}

fn rule_match_only(c: &mut Criterion) {
  
  let rules = default_lists();
  let requests = load_requests();
  let requests_parsed: Vec<_> = requests.into_iter().map(|r| { Request::from_urls(&r.url, &r.frameUrl, &r.cpt) }).filter_map(Result::ok).collect();
  let requests_len = requests_parsed.len() as u32;
  c.bench(
        "rule-match-parsed",
        Benchmark::new(
            "easylist",
            move |b| {
              let blocker = get_blocker(&rules);
              b.iter(|| bench_matching_only(&blocker, &requests_parsed))
            },
        ).throughput(Throughput::Elements(requests_len))
        .sample_size(10)
    );
}

fn rule_match_only_el_ep(c: &mut Criterion) {
  
  let rules = rules_from_lists(vec![
    "data/easylist.to/easylist/easylist.txt",
    "data/easylist.to/easylist/easyprivacy.txt"
  ]);
  let requests = load_requests();
  let requests_parsed: Vec<_> = requests.into_iter().map(|r| { Request::from_urls(&r.url, &r.frameUrl, &r.cpt) }).filter_map(Result::ok).collect();
  let requests_len = requests_parsed.len() as u32;
  let blocker = get_blocker(&rules);
  c.bench(
        "rule-match-parsed",
        Benchmark::new(
            "el+ep",
            move |b| {
              b.iter(|| bench_matching_only(&blocker, &requests_parsed))
            },
        ).throughput(Throughput::Elements(requests_len))
        .sample_size(5)
    );
}

criterion_group!(benches, rule_match_only_el_ep, rule_match_only, rule_match);
criterion_main!(benches);
