extern crate criterion;

use criterion::*;

use serde::{Deserialize, Serialize};
use serde_json;

use adblock;
use adblock::utils::{read_rules, rules_from_lists};
use adblock::blocker::{Blocker, BlockerOptions};


fn default_lists() -> Vec<String> {
  rules_from_lists(vec![
    "data/easylist.to/easylist/easylist.txt",
    // "data/easylist.to/easylist/easyprivacy.txt"
  ])
}

fn default_rules_lists() -> Vec<Vec<String>> {
  vec![
    read_rules("data/easylist.to/easylist/easylist.txt"),
    // read_rules("data/easylist.to/easylist/easyprivacy.txt")
  ]
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
      let req: Result<adblock::request::Request, _> = adblock::request::Request::from_urls(&r.url, &r.frameUrl, &r.cpt);
      match req.map(|parsed| blocker.check(&parsed)).as_ref() {
        Ok(check) if check.matched => matches += 1,
        Ok(_) => passes += 1,
        Err(_) => errors += 1
      }
    });
  (matches, passes, errors)
}

fn rule_match(c: &mut Criterion) {
  
  let rules = default_lists();
  let requests = load_requests();
  let requests_len = requests.len() as u32;
  c.bench(
        "parse-filters",
        Benchmark::new(
            "network filters",
            move |b| {
              let blocker = get_blocker(&rules);
              b.iter(|| bench_rule_matching(&blocker, &requests))
            },
        ).throughput(Throughput::Elements(requests_len))
        .sample_size(10)
    );
}

criterion_group!(benches, rule_match);
criterion_main!(benches);
