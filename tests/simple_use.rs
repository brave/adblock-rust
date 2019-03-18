extern crate adblock;

use adblock::lists::parse_filters;
use adblock::blocker::{Blocker, BlockerOptions};
use adblock::request::Request;

#[test]
fn check_simple_use() {
    let rules = vec![
        String::from("-advertisement-icon."),
        String::from("-advertisement-management/"),
        String::from("-advertisement."),
        String::from("-advertisement/script."),
    ];
    let (network_filters, _) = parse_filters(&rules, true, false, false);

    let blocker_options = BlockerOptions {
        debug: false,
        enable_optimizations: false,
        load_cosmetic_filters: false,
        load_network_filters: true
    };
    
    let blocker = Blocker::new(network_filters, &blocker_options);

    let maybeReq = Request::from_urls("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image");

    assert!(maybeReq.is_ok(), "Request failed to parse");
    let req = maybeReq.unwrap();
    let blocker_result = blocker.check(&req);
    assert!(blocker_result.matched);
}