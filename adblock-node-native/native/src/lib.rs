#[macro_use]
extern crate neon;

use neon::prelude::*;

extern crate adblock;

use adblock::blocker::Blocker;
use adblock::request::Request;

declare_types! {
    pub class JsBlocker for Blocker {
        init(mut cx) {
            // Take the first argument, which must be an array
            let rules_handle: Handle<JsArray> = cx.argument(0)?;
            // Convert a JsArray to a Rust Vec
            let rules_wrapped: Vec<_> = rules_handle.to_vec(&mut cx)?;

            let mut rules: Vec<String> = vec![];
            for rule_wrapped in rules_wrapped {
                let rule = rule_wrapped.downcast::<JsString>().or_throw(&mut cx)?
                    .value();
                rules.push(rule);
            }

            Ok(blocker_new(&rules))
        }

        method block(mut cx) {
            let url: String = cx.argument::<JsString>(0)?.value();
            let source_url: String = cx.argument::<JsString>(1)?.value();
            let request_type: String = cx.argument::<JsString>(2)?.value();

            let this = cx.this();

            let request = Request::from_urls(&url, &source_url, &request_type).unwrap();

            let result = {
                let guard = cx.lock();
                let blocker = this.borrow(&guard);
                blocker.check(&request)
            };
            Ok(cx.boolean(result.matched).upcast())
        }
    }
}

register_module!(mut m, {
    // Export the `JsBlocker` class
    m.export_class::<JsBlocker>("Blocker")?;
    Ok(())
});

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