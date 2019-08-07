extern crate adblock;

use adblock::engine::Engine;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    // Empty engine
    let mut engine = Engine::from_rules(&[]);

    // Read serialized version
    let mut file = File::open("engine.dat").unwrap();
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer).unwrap();
    
    // Deserialize
    engine.deserialize(&buffer);
    engine.with_tags(&["twitter-embeds"]);
    assert!(engine.check_network_urls("https://platform.twitter.com/widgets.js", "https://fmarier.github.io/brave-testing/social-widgets.html", "script").filter.is_some());
}
