use wasm_bindgen::prelude::*;

extern crate adblock;
extern crate serde_json;

use adblock::blocker::Blocker;
use adblock::request::Request;


fn blocker_new(rules: &Vec<String>) -> Blocker {
    let (network_filters, _) = adblock::lists::parse_filters(&rules, true, false, false);

    let blocker_options = adblock::blocker::BlockerOptions {
        debug: false,
        enable_optimizations: true,
        load_cosmetic_filters: false,
        load_network_filters: true
    };

    adblock::blocker::Blocker::new(network_filters, &blocker_options)
}

#[wasm_bindgen]
pub struct JsBlocker {
    blocker: Blocker   
}

#[wasm_bindgen]
impl JsBlocker {
    pub fn new(rules_js: &JsValue) -> JsBlocker {
        // let rules: Vec<String> = rules_js.into_serde().unwrap();
        let blocker = blocker_new(&Vec::new());
        JsBlocker {
            blocker
        }
    }

    pub fn check(&self, url: &str, source_url: &str, request_type: &str) -> bool {
        let request = Request::from_urls(url, source_url, request_type).unwrap();
        let result = self.blocker.check(&request);
        result.matched
    }
}