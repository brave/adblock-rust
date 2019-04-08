extern crate criterion;

use criterion::*;

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


fn bench_string_hashing(filters: &Vec<String>) -> adblock::utils::Hash {
  let mut dummy: adblock::utils::Hash = 0;
  for filter in filters {
    dummy = (dummy + adblock::utils::fast_hash(filter)) % 1000000000;
  }
  dummy
}

fn bench_string_tokenize(filters: &Vec<String>) -> usize {
  let mut dummy: usize = 0;
  for filter in filters {
    dummy = (dummy + adblock::utils::tokenize(filter).len()) % 1000000000;
  }
  dummy
}


fn string_hashing(c: &mut Criterion) {
  let rules = default_lists();
  c.bench(
        "string-hashing",
        Benchmark::new(
            "hash",
            move |b| b.iter(|| bench_string_hashing(&rules)),
        ).throughput(Throughput::Elements(1)),
    );
}

fn string_tokenize(c: &mut Criterion) {
  let rules = default_lists();
  c.bench(
        "string-tokenize",
        Benchmark::new(
            "tokenize",
            move |b| b.iter(|| bench_string_tokenize(&rules)),
        ).throughput(Throughput::Elements(1)),
    );
}

fn bench_parsing_impl(lists: &Vec<Vec<String>>, load_network_filters: bool, load_cosmetic_filters: bool) -> usize {
  let mut dummy = 0;

  for list in lists {
      let (network_filters, _) = adblock::lists::parse_filters(list, load_network_filters, load_cosmetic_filters, false);
      dummy = dummy + network_filters.len() % 1000000;
  }
  
  dummy
}

fn list_parse(c: &mut Criterion) {
  let rules_lists = default_rules_lists();
  c.bench(
        "parse-filters",
        Benchmark::new(
            "network filters",
            move |b| b.iter(|| bench_parsing_impl(&rules_lists, true, false)),
        ).throughput(Throughput::Elements(1))
        .sample_size(10)
    );
}


fn get_blocker(rules: &Vec<String>) -> Blocker {
  let (network_filters, _) = adblock::lists::parse_filters(rules, true, false, false);

  println!("Got {} network filters", network_filters.len());

  let blocker_options = BlockerOptions {
    debug: false,
    enable_optimizations: true,
    load_cosmetic_filters: false,
    load_network_filters: true
  };
  
  Blocker::new(network_filters, &blocker_options)
}


fn blocker_new(c: &mut Criterion) {
  let rules = rules_from_lists(vec![
    "data/easylist.to/easylist/easylist.txt",
    "data/easylist.to/easylist/easyprivacy.txt"
  ]);

  c.bench(
        "blocker_new",
        Benchmark::new(
            "el+ep",
            move |b| b.iter(|| get_blocker(&rules)),
        ).throughput(Throughput::Elements(1))
        .sample_size(10)
    );
}



criterion_group!(benches, blocker_new, list_parse, string_hashing, string_tokenize);
criterion_main!(benches);
