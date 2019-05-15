extern crate wasm_bindgen;
use wasm_bindgen::prelude::*;

extern crate js_sys;
extern crate adblock;
extern crate serde_json;

use adblock::engine::Engine;


fn blocker_new(rules: &Vec<String>, rules_debug: bool) -> Engine {
    if rules_debug {
        Engine::from_rules_debug(&rules)
    } else {
        Engine::from_rules(&rules)
    }
}

#[wasm_bindgen]
pub struct JsBlocker {
    engine: Engine
}

#[wasm_bindgen]
impl JsBlocker {
    pub fn new(rules_js: &JsValue) -> JsBlocker {
        let iterator = js_sys::try_iter(rules_js).unwrap().unwrap();

        let mut rules = Vec::new();
        for x in iterator {
            let x = x.unwrap();

            // If `x` is a string, add it to our array of rules
            if x.is_string() {
                let rule = x.as_string().unwrap();
                rules.push(rule);
            }
        }
        let engine = blocker_new(&rules, false);
        JsBlocker {
            engine
        }
    }

    pub fn check(&self, url: &str, source_url: &str, request_type: &str) -> bool {
        let result = self.engine.check_network_urls(&url, &source_url, &request_type);
        result.matched
    }
}
