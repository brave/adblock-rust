use adblock::{Engine, request::Request};
use std::fs;

#[test]
fn measure_serialized_size() {
    // Load the actual Brave main list
    let brave_list_content = fs::read_to_string("data/brave/brave-main-list.txt")
        .expect("Failed to read brave-main-list.txt");

    let rules: Vec<&str> = brave_list_content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('!'))
        .collect();

    println!("Loading {} rules from brave-main-list.txt", rules.len());

    let engine = Engine::from_rules_parametrised(
        rules,
        Default::default(),
        false,
        false,
    );

    let serialized = engine.serialize().unwrap();
    let size_mb = serialized.len() as f64 / (1024.0 * 1024.0);

    println!("Serialized data size: {:.2} MB ({} bytes)", size_mb, serialized.len());

    // Verify deserialization works
    let mut engine2 = Engine::new(false);
    engine2.deserialize(&serialized).unwrap();

    // Basic functionality test
    let request = Request::new(
        "https://googlesyndication.com/script.js",
        "https://example.com",
        "script"
    ).unwrap();

    let result1 = engine.check_network_request(&request);
    let result2 = engine2.check_network_request(&request);

    println!("Original engine result: {}", result1.matched);
    println!("Deserialized engine result: {}", result2.matched);

    // Results should be the same
    assert_eq!(result1.matched, result2.matched);
}
