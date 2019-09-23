extern crate adblock;
extern crate reqwest;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::engine::Engine;
use adblock::filters::network::NetworkFilter;
use adblock::utils;

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
    blocked: bool
}

fn load_requests() -> Vec<RequestRuleMatch> {
    let f = File::open("data/regressions.tsv").expect("file not found");
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

fn get_blocker_engine() -> Engine {
  let network_filters: Vec<NetworkFilter> = adblock::filter_lists::default::default_lists()
        .iter()
        .map(|list| {
            let filters: Vec<String> = reqwest::get(&list.url).expect("Could not request rules")
                .text().expect("Could not get rules as text")
                .lines()
                .map(|s| s.to_owned())
                .collect();

            let (network_filters, _) = adblock::lists::parse_filters(&filters, true, false, true);
            network_filters
        })
        .flatten()
        .collect();

  let blocker_options = BlockerOptions {
    debug: true,
    enable_optimizations: false,
    load_cosmetic_filters: false,
    load_network_filters: true
  };
  
    let mut engine = Engine {
        blocker: Blocker::new(network_filters, &blocker_options)
    };

    engine.with_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn get_blocker_engine_deserialized() -> Engine {
    let dat_url = "https://adblock-data.s3.amazonaws.com/4/rs-ABPFilterParserData.dat";
    let mut dat: Vec<u8> = vec![];
    let mut resp = reqwest::get(dat_url).expect("Could not request rules");
    resp.copy_to(&mut dat).expect("Could not copy response to byte array");

    let mut engine = Engine::from_rules(&[]);
    engine.deserialize(&dat).expect("Deserialization failed");
    engine.with_tags(&["fb-embeds", "twitter-embeds"]);
    engine
}

#[test]
fn check_live_specific_urls() {
    let engine = get_blocker_engine();
    {
        let checked = engine.check_network_urls(
            "https://static.scroll.com/js/scroll.js",
            "https://www.theverge.com/",
            "script");
        assert_eq!(checked.matched, false,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
    }
}

#[test]
fn check_live_from_filterlists() {
    let engine = get_blocker_engine();
    let requests = load_requests();
    
    for req in requests {
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} at {}, got filter {:?}, exception {:?}",
            req.blocked, req.url, req.sourceUrl, checked.filter, checked.exception);
    }
}

#[test]
fn check_live_deserialized() {
    let engine = get_blocker_engine_deserialized();
    let requests = load_requests();
    
    for req in requests {
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} {} {}",
            req.blocked, req.url, req.sourceUrl, req.r#type);
    }
}

#[test]
fn check_live_redirects() {
    let mut engine = get_blocker_engine();
    let resources_lines = utils::read_file_lines("data/uBlockOrigin/resources.txt");
    let resources_str = resources_lines.join("\n");
    engine.with_resources(&resources_str);
    { 
        let checked = engine.check_network_urls(
            "https://c.amazon-adsystem.com/aax2/amzn_ads.js",
            "https://aussieexotics.com/",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        assert!(checked.redirect.is_some());
        // Check for the specific expected return script value in base64
        assert_eq!(checked.redirect.unwrap(), "data:application/javascript;base64,KGZ1bmN0aW9uKCkgewoJaWYgKCBhbXpuYWRzICkgewoJCXJldHVybjsKCX0KCXZhciB3ID0gd2luZG93OwoJdmFyIG5vb3BmbiA9IGZ1bmN0aW9uKCkgewoJCTsKCX0uYmluZCgpOwoJdmFyIGFtem5hZHMgPSB7CgkJYXBwZW5kU2NyaXB0VGFnOiBub29wZm4sCgkJYXBwZW5kVGFyZ2V0aW5nVG9BZFNlcnZlclVybDogbm9vcGZuLAoJCWFwcGVuZFRhcmdldGluZ1RvUXVlcnlTdHJpbmc6IG5vb3BmbiwKCQljbGVhclRhcmdldGluZ0Zyb21HUFRBc3luYzogbm9vcGZuLAoJCWRvQWxsVGFza3M6IG5vb3BmbiwKCQlkb0dldEFkc0FzeW5jOiBub29wZm4sCgkJZG9UYXNrOiBub29wZm4sCgkJZGV0ZWN0SWZyYW1lQW5kR2V0VVJMOiBub29wZm4sCgkJZ2V0QWRzOiBub29wZm4sCgkJZ2V0QWRzQXN5bmM6IG5vb3BmbiwKCQlnZXRBZEZvclNsb3Q6IG5vb3BmbiwKCQlnZXRBZHNDYWxsYmFjazogbm9vcGZuLAoJCWdldERpc3BsYXlBZHM6IG5vb3BmbiwKCQlnZXREaXNwbGF5QWRzQXN5bmM6IG5vb3BmbiwKCQlnZXREaXNwbGF5QWRzQ2FsbGJhY2s6IG5vb3BmbiwKCQlnZXRLZXlzOiBub29wZm4sCgkJZ2V0UmVmZXJyZXJVUkw6IG5vb3BmbiwKCQlnZXRTY3JpcHRTb3VyY2U6IG5vb3BmbiwKCQlnZXRUYXJnZXRpbmc6IG5vb3BmbiwKCQlnZXRUb2tlbnM6IG5vb3BmbiwKCQlnZXRWYWxpZE1pbGxpc2Vjb25kczogbm9vcGZuLAoJCWdldFZpZGVvQWRzOiBub29wZm4sCgkJZ2V0VmlkZW9BZHNBc3luYzogbm9vcGZuLAoJCWdldFZpZGVvQWRzQ2FsbGJhY2s6IG5vb3BmbiwKCQloYW5kbGVDYWxsQmFjazogbm9vcGZuLAoJCWhhc0Fkczogbm9vcGZuLAoJCXJlbmRlckFkOiBub29wZm4sCgkJc2F2ZUFkczogbm9vcGZuLAoJCXNldFRhcmdldGluZzogbm9vcGZuLAoJCXNldFRhcmdldGluZ0ZvckdQVEFzeW5jOiBub29wZm4sCgkJc2V0VGFyZ2V0aW5nRm9yR1BUU3luYzogbm9vcGZuLAoJCXRyeUdldEFkc0FzeW5jOiBub29wZm4sCgkJdXBkYXRlQWRzOiBub29wZm4KCX07Cgl3LmFtem5hZHMgPSBhbXpuYWRzOwoJdy5hbXpuX2FkcyA9IHcuYW16bl9hZHMgfHwgbm9vcGZuOwoJdy5hYXhfd3JpdGUgPSB3LmFheF93cml0ZSB8fCBub29wZm47Cgl3LmFheF9yZW5kZXJfYWQgPSB3LmFheF9yZW5kZXJfYWQgfHwgbm9vcGZuOwp9KSgpOw==")
    }
    {
        let checked = engine.check_network_urls(
            "https://www.googletagservices.com/tag/js/gpt.js",
            "https://winniethepooh.disney.com/",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        assert!(checked.redirect.is_some());
    }
    
}