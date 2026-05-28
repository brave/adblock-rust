use adblock::request::Request;
use adblock::Engine;

#[test]
fn check_simple_use() {
    let rules = [
        "-advertisement-icon.",
        "-advertisement-management/",
        "-advertisement.",
        "-advertisement/script.",
    ];

    let engine = Engine::new_with_list_text(rules.join("\n"), Default::default());

    let request = Request::new(
        "http://example.com/-advertisement-icon.",
        "http://example.com/helloworld",
        "image",
    )
    .unwrap();
    let blocker_result = engine.check_network_request(&request);
    assert!(blocker_result.matched);
}
