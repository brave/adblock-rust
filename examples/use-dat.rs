extern crate adblock;

use adblock::engine::Engine;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    // Empty engine
    let mut engine = Engine::default();

    // Read serialized version
    let mut file = File::open("engine.dat").unwrap();
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer).unwrap();
    
    // Deserialize
    engine.deserialize(&buffer).unwrap();
    engine.use_tags(&["twitter-embeds"]);
    let checked = engine.check_network_urls("https://platform.twitter.com/widgets.js", "https://fmarier.github.io/brave-testing/social-widgets.html", "script");
    assert!(checked.filter.is_some());
    assert!(checked.exception.is_some());
    println!("All good: {:?}", checked);
}
