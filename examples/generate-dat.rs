use adblock::{request::Request, Engine};

use std::fs::File;
use std::io::prelude::*;

fn main() {
    // Rules we want to serialize
    let rules = vec![
        String::from("||platform.twitter.com/$tag=twitter-embeds"),
        String::from("@@||platform.twitter.com/$tag=twitter-embeds"),
    ];

    // Serialize
    let mut engine = Engine::from_rules_debug(&rules, Default::default());
    engine.use_tags(&["twitter-embeds"]);

    let request = Request::new(
        "https://platform.twitter.com/widgets.js",
        "https://fmarier.github.io/brave-testing/social-widgets.html",
        "script",
    )
    .unwrap();
    assert!(engine.check_network_request(&request).exception.is_some());
    let serialized = engine.serialize().expect("Could not serialize!");

    // Write to file
    let mut file = File::create("engine.dat").expect("Could not create serialization file");
    file.write_all(&serialized)
        .expect("Could not output serialized engine to file");
}
