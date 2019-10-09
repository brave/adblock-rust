extern crate adblock;

use adblock::engine::Engine;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::filters::network::NetworkFilter;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::{thread, time};
use adblock::lists::FilterList;
use csv;

fn get_blocker_engine(filter_lists: &Vec<FilterList>) -> Engine {
    let network_filters: Vec<NetworkFilter> = filter_lists
        .iter()
        .map(|list| {
            let filters: Vec<String> = reqwest::get(&list.url)
                .expect("Could not request rules")
                .text()
                .expect("Could not get rules as text")
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
        load_network_filters: true,
    };

    let mut engine = Engine {
        blocker: Blocker::new(network_filters, &blocker_options),
    };

    engine.with_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn main() {
    println!("Deserializing engine");
    let mut engine;
    {
        // let mut file = File::open("data/rs-ABPFilterParserData.dat").expect("Opening serialization file failed");
        // let metadata = fs::metadata("data/rs-ABPFilterParserData.dat").expect("Getting file metadata failed");
        // let mut serialized = Vec::<u8>::with_capacity(metadata.len() as usize);
        // file.read_to_end(&mut serialized).expect("Reading from serialization file failed");
        // serialized.shrink_to_fit();
        // println!("Serialized size {:?}", serialized.len());
        engine = get_blocker_engine(&adblock::filter_lists::default::default_lists());

        // engine.deserialize(&serialized).expect("Deserialization failed");
    }
    engine.with_tags(&["twitter-embeds"]);

    let sleeptime = time::Duration::from_secs(3);
    thread::sleep(sleeptime);
}
