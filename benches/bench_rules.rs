extern crate criterion;

use criterion::*;
use lazy_static::lazy_static;

use adblock;
use adblock::utils::{read_file_lines, rules_from_lists};
use adblock::blocker::{Blocker, BlockerOptions};


lazy_static! {
    static ref DEFAULT_LISTS: Vec<String> = rules_from_lists(&vec![
        String::from("data/easylist.to/easylist/easylist.txt"),
    ]);
    static ref DEFAULT_RULES_LISTS: Vec<Vec<String>> = vec![
        read_file_lines("data/easylist.to/easylist/easylist.txt"),
    ];
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
  c.bench(
        "string-hashing",
        Benchmark::new(
            "hash",
            move |b| b.iter(|| bench_string_hashing(&DEFAULT_LISTS)),
        ).throughput(Throughput::Elements(1)),
    );
}

fn string_tokenize(c: &mut Criterion) {
  c.bench(
        "string-tokenize",
        Benchmark::new(
            "tokenize",
            move |b| b.iter(|| bench_string_tokenize(&DEFAULT_LISTS)),
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
  c.bench(
        "parse-filters",
        Benchmark::new(
            "network filters",
            |b| b.iter(|| bench_parsing_impl(&DEFAULT_RULES_LISTS, true, false)),
        ).with_function(
            "all filters",
            |b| b.iter(|| bench_parsing_impl(&DEFAULT_RULES_LISTS, true, true)),
        )
        .throughput(Throughput::Elements(1))
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
  let rules = rules_from_lists(&vec![
    String::from("data/easylist.to/easylist/easylist.txt"),
    String::from("data/easylist.to/easylist/easyprivacy.txt")
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
