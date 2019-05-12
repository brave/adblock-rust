#[macro_use]
extern crate neon;

extern crate neon_serde;
extern crate adblock;

use neon::prelude::*;
use adblock::engine::Engine;
use adblock::filter_lists;

declare_types! {
    pub class JsEngine for Engine {
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

            Ok(Engine::from_rules(&rules))
        }

        method check(mut cx) {
            let url: String = cx.argument::<JsString>(0)?.value();
            let source_url: String = cx.argument::<JsString>(1)?.value();
            let request_type: String = cx.argument::<JsString>(2)?.value();

            let this = cx.this();

            let result = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.check_network_urls(&url, &source_url, &request_type)
            };
            Ok(cx.boolean(result.matched).upcast())
        }

        method serialize(mut cx) {
            let this = cx.this();
            let serialized = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.serialize().unwrap()
            };

            // initialise new Array Buffer in the JS context
            let mut buffer = JsArrayBuffer::new(&mut cx, serialized.len() as u32)?;
            // copy data from Rust buffer to JS Array Buffer
            cx.borrow_mut(&mut buffer, |bufferdata| {
                let slice = bufferdata.as_mut_slice::<u8>();
                slice.copy_from_slice(&serialized)
            });
            
            Ok(buffer.upcast())
        }

        method deserialize(mut cx) {
            let serialized_handle = cx.argument::<JsArrayBuffer>(0)?;
            let mut this = cx.this();
            let guard = cx.lock();
            let _result = cx.borrow(&serialized_handle, |bufferdata| {
                let slice = bufferdata.as_slice::<u8>();
                let mut engine = this.borrow_mut(&guard);
                engine.deserialize(&slice)
            }).unwrap();

            Ok(JsNull::new().upcast())
        }
    }
}

fn lists(mut cx: FunctionContext) -> JsResult<JsValue> {
    let category: String = cx.argument::<JsString>(0)?.value();
    let filter_list: Vec<adblock::lists::FilterList>;
    if category == "regions" {
        filter_list = filter_lists::regions::regions();
    } else {
        filter_list = filter_lists::default::default_lists();
    }

    let js_list = neon_serde::to_value(&mut cx, &filter_list)?;

    Ok(js_list)
}

register_module!(mut m, {
    // Export the `JsEngine` class
    m.export_class::<JsEngine>("Engine")?;
    m.export_function("lists", lists)?;
    Ok(())
});
