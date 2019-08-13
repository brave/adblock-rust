extern crate adblock;
extern crate reqwest;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::engine::Engine;
use adblock::filters::network::NetworkFilter;

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