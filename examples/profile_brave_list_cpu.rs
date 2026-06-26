//! CPU-only profiling (no dhat overhead) for issue #541.
//!
//!   cargo run --release --example profile_brave_list_cpu

use adblock::{Engine, FilterSet};
use std::time::Instant;

fn load_shields_style() {
    let list_text = std::fs::read_to_string("data/brave/brave-main-list.txt")
        .expect("brave-main-list.txt");

    let t0 = Instant::now();
    let mut filter_set = FilterSet::new(false);
    filter_set.add_filter_list(&list_text, adblock::lists::ParseOptions::default());
    let parse_done = Instant::now();

    let engine = Engine::from_filter_set(filter_set, true);
    let compile_done = Instant::now();

    std::hint::black_box(engine.serialize());
    let finish = Instant::now();

    eprintln!("add_filter_list:        {:>8.1} ms", (parse_done - t0).as_secs_f64() * 1000.0);
    eprintln!("from_filter_set:        {:>8.1} ms", (compile_done - parse_done).as_secs_f64() * 1000.0);
    eprintln!("serialize:              {:>8.1} ms", (finish - compile_done).as_secs_f64() * 1000.0);
    eprintln!("TOTAL:                  {:>8.1} ms", (finish - t0).as_secs_f64() * 1000.0);
}

fn load_from_rules_style() {
    let rules: Vec<String> = std::fs::read_to_string("data/brave/brave-main-list.txt")
        .expect("brave-main-list.txt")
        .lines()
        .map(str::to_string)
        .collect();

    let t0 = Instant::now();
    let engine = Engine::from_rules(&rules, Default::default());
    let finish = Instant::now();

    std::hint::black_box(engine.serialize());
    eprintln!("Engine::from_rules TOTAL: {:>8.1} ms", (finish - t0).as_secs_f64() * 1000.0);
}

fn main() {
    eprintln!("=== Shields path (add_filter_list) ===");
    load_shields_style();
    eprintln!();
    eprintln!("=== from_rules path ===");
    load_from_rules_style();
}
