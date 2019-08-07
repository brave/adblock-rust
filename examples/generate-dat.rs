extern crate adblock;

use adblock::engine::Engine;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    // Rules we want to serialize
    let rules = vec![
        String::from("||platform.twitter.com/$tag=twitter-embeds"),
        String::from("@@||platform.twitter.com/$tag=twitter-embeds")
    ];

    // Serialize
    let mut engine = Engine::from_rules_debug(&rules);
    engine.with_tags(&["twitter-embeds"]);    
    assert!(engine.check_network_urls("https://platform.twitter.com/widgets.js", "https://fmarier.github.io/brave-testing/social-widgets.html", "script").exception.is_some());
    let serialized = engine.serialize().unwrap();
    
    // Write to file
    let mut file = File::create("engine.dat").unwrap();
    file.write_all(&serialized).unwrap();
}
