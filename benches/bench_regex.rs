extern crate criterion;

use criterion::*;

use regex::Regex;
use regex::bytes::{Regex as BytesRegex};
use regex::RegexSet;

fn bench_simple_regexes(c: &mut Criterion) {
  let pattern = "?/static/adv/foobar/asd?q=1";

  let rules = vec![
      Regex::new(r"(?:[^\\w\\d\\._%-])/static/ad-").unwrap(),
      Regex::new(r"(?:[^\\w\\d\\._%-])/static/ad/.*").unwrap(),
      Regex::new(r"(?:[^\\w\\d\\._%-])/static/ads/.*").unwrap(),
      Regex::new(r"(?:[^\\w\\d\\._%-])/static/adv/.*").unwrap(),
  ];

  c.bench(
        "regex",
        Benchmark::new(
            "list",
            move |b| {
              b.iter(|| {
                
                for rule in rules.iter() {
                  if rule.is_match(&pattern) {
                    true;
                  } else {
                    false;
                  }
                }
                
              })
            },
        )
    );
}

fn bench_joined_regex(c: &mut Criterion) {
  let pattern = "?/static/adv/foobar/asd?q=1";

  let rule = Regex::new(r"(?:([^\\w\\d\\._%-])/static/ad-)|(?:([^\\w\\d\\._%-])/static/ad/.*)(?:([^\\w\\d\\._%-])/static/ads/.*)(?:([^\\w\\d\\._%-])/static/adv/.*)").unwrap();

  c.bench(
        "regex",
        Benchmark::new(
            "joined",
            move |b| {
              b.iter(|| rule.is_match(&pattern))
            },
        )
    );
}

fn bench_joined_bytes_regex(c: &mut Criterion) {
  let pattern = "?/static/adv/foobar/asd?q=1";

  let rule = BytesRegex::new(r"(?:([^\\w\\d\\._%-])/static/ad-)|(?:([^\\w\\d\\._%-])/static/ad/.*)(?:([^\\w\\d\\._%-])/static/ads/.*)(?:([^\\w\\d\\._%-])/static/adv/.*)").unwrap();

  c.bench(
        "regex",
        Benchmark::new(
            "u8",
            move |b| {
              b.iter(|| rule.is_match(pattern.as_bytes()))
            },
        )
    );
}

fn bench_regex_set(c: &mut Criterion) {
  let pattern = "?/static/adv/foobar/asd?q=1";

  let set = RegexSet::new(&[
      r"(?:[^\\w\\d\\._%-])/static/ad-",
      r"(?:[^\\w\\d\\._%-])/static/ad/.*",
      r"(?:[^\\w\\d\\._%-])/static/ads/.*",
      r"(?:[^\\w\\d\\._%-])/static/adv/.*",
  ]).unwrap();

  c.bench(
        "regex",
        Benchmark::new(
            "set",
            move |b| {
              b.iter(|| set.is_match(&pattern))
            },
        )
    );
}


criterion_group!(benches, bench_simple_regexes, bench_joined_regex, bench_joined_bytes_regex, bench_regex_set);
criterion_main!(benches);
