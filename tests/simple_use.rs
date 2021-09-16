use adblock::engine::Engine;

#[test]
fn check_simple_use() {
    let rules = vec![
        String::from("-advertisement-icon."),
        String::from("-advertisement-management/"),
        String::from("-advertisement."),
        String::from("-advertisement/script."),
    ];

    let blocker = Engine::from_rules(&rules, Default::default());
    let blocker_result = blocker.check_network_urls("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image");
    assert!(blocker_result.matched);
}
