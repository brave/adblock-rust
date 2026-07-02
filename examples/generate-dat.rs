use adblock::{request::Request, Engine};

use std::fs::File;
use std::io::prelude::*;

fn main() {
    // Rules we want to serialize
    let rules = [
        String::from("||platform.twitter.com^"),
        String::from("@@||platform.twitter.com^"),
    ]
    .join("\n");

    // Serialize
    let engine = Engine::new_with_list_text(rules);

    let request = Request::new(
        "https://platform.twitter.com/widgets.js",
        "https://fmarier.github.io/brave-testing/social-widgets.html",
        "script",
        "",
    )
    .unwrap();
    assert!(engine.check_network_request(&request).exception.is_some());
    let serialized = engine.serialize().to_vec();

    // Write to file
    let mut file = File::create("engine.dat").expect("Could not create serialization file");
    file.write_all(&serialized)
        .expect("Could not output serialized engine to file");
}
