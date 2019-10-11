extern crate adblock;

use adblock::engine::Engine;
use adblock::blocker::{Blocker, BlockerOptions};
use adblock::filters::network::NetworkFilter;
use std::fs::File;
use std::io::prelude::*;

fn get_blocker_engine() -> Engine {
  let network_filters: Vec<NetworkFilter> = adblock::filter_lists::slimlist::slim_list()
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
        debug: false,
        enable_optimizations: true,
        load_cosmetic_filters: false,
        load_network_filters: true
    };
  
    let mut engine = Engine {
        blocker: Blocker::new(network_filters, &blocker_options)
    };

    engine.with_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn main() {
    // Rules we want to serialize
    // let engine = get_blocker_engine();  
    // Rules we want to serialize
    let rules = vec![
        String::from("/beacon.js"),
        // String::from("@@||platform.twitter.com/$tag=twitter-embeds")
    ];

    // Serialize
    let mut engine = Engine::from_rules_debug(&rules);
    // engine.with_tags(&["twitter-embeds"]);    
    // assert!(engine.check_network_urls("https://platform.twitter.com/widgets.js", "https://fmarier.github.io/brave-testing/social-widgets.html", "script").exception.is_some());
    // let serialized = engine.serialize().expect("Could not serialize!");


    // assert!(engine.check_network_urls("https://platform.twitter.com/widgets.js", "https://fmarier.github.io/brave-testing/social-widgets.html", "script").exception.is_some());
    let serialized = engine.serialize().expect("Could not serialize!");

    // Write to file
    let mut file = File::create("engine.dat").expect("Could not create serialization file");
    file.write_all(&serialized).expect("Could not output serialized engine to file");
}
