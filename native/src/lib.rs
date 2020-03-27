extern crate neon;
extern crate neon_serde;
extern crate adblock;
extern crate serde;

use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use adblock::engine::Engine;
use adblock::filter_lists;
use adblock::resources::Resource;
use adblock::resources::resource_assembler::{assemble_web_accessible_resources, assemble_scriptlet_resources};


#[derive(Serialize, Deserialize)]
struct EngineOptions {
    pub debug: Option<bool>,
    pub optimize: Option<bool>,
    pub loadNetwork: Option<bool>,
    pub loadCosmetic: Option<bool>,
}

declare_types! {
    pub class JsEngine for Engine {
        init(mut cx) {
            // Take the first argument, which must be an array
            let rules_handle: Handle<JsArray> = cx.argument(0)?;

            let debug: bool;
            let optimize: bool;
            let load_network: bool;
            let load_cosmetic: bool;
            match cx.argument_opt(1) {
                Some(arg) => {
                    // Throw if the argument exist and it cannot be downcasted to a boolean
                    let maybe_config: Result<EngineOptions, _> = neon_serde::from_value(&mut cx, arg);
                    if let Ok(config) = maybe_config {
                        debug = config.debug.unwrap_or(false);
                        optimize = config.optimize.unwrap_or(true);
                        load_network = config.loadNetwork.unwrap_or(true);
                        load_cosmetic = config.loadCosmetic.unwrap_or(true);
                    } else {
                        debug = arg.downcast::<JsBoolean>().or_throw(&mut cx)?.value();
                        optimize = true;
                        load_network = true;
                        load_cosmetic = true;
                    }
                }
                None => {
                    debug = false;
                    optimize = true;
                    load_network = true;
                    load_cosmetic = true;
                },
            }
            // Convert a JsArray to a Rust Vec
            let rules_wrapped: Vec<_> = rules_handle.to_vec(&mut cx)?;

            let mut rules: Vec<String> = vec![];
            for rule_wrapped in rules_wrapped {
                let rule = rule_wrapped.downcast::<JsString>().or_throw(&mut cx)?
                    .value();
                rules.push(rule);
            }

            Ok(Engine::from_rules_parametrised(&rules, load_network, load_cosmetic, debug, optimize))
        }

        method check(mut cx) {
            let url: String = cx.argument::<JsString>(0)?.value();
            let source_url: String = cx.argument::<JsString>(1)?.value();
            let request_type: String = cx.argument::<JsString>(2)?.value();

            let debug = match cx.argument_opt(3) {
                Some(arg) => {
                    // Throw if the argument exist and it cannot be downcasted to a boolean
                    arg.downcast::<JsBoolean>().or_throw(&mut cx)?.value()
                }
                None => false,
            };

            let this = cx.this();

            let result = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.check_network_urls(&url, &source_url, &request_type)
            };
            if debug {
                let js_value = neon_serde::to_value(&mut cx, &result)?;
                Ok(js_value)
            } else {
                Ok(cx.boolean(result.matched).upcast())
            }
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

        method enableTag(mut cx) {
            let tag: String = cx.argument::<JsString>(0)?.value();

            let mut this = cx.this();
            let guard = cx.lock();
            let _result = {
                let mut engine = this.borrow_mut(&guard);
                engine.tags_enable(&[&tag])
            };
            Ok(JsNull::new().upcast())
        }

        method updateResources(mut cx) {
            let resources_arg = cx.argument::<JsValue>(0)?;
            let resources: Vec<Resource> = neon_serde::from_value(&mut cx, resources_arg)?;

            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut engine = this.borrow_mut(&guard);
                engine.with_resources(&resources);
            }
            Ok(JsNull::new().upcast())

        }
        method tagExists(mut cx) {
            let tag: String = cx.argument::<JsString>(0)?.value();

            let this = cx.this();
            let result = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.tag_exists(&tag)
            };
            Ok(cx.boolean(result).upcast())
        }

        method clearTags(mut cx) {
            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut engine = this.borrow_mut(&guard);
                // enabling an empty list of tags disables all tags
                engine.tags_enable(&[]);
            }
            Ok(JsNull::new().upcast())
        }

        method addFilter(mut cx) {
            let filter: String = cx.argument::<JsString>(0)?.value();

            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut engine = this.borrow_mut(&guard);
                engine.filter_add(&filter);
            }
            Ok(JsNull::new().upcast())
        }

        method addResource(mut cx) {
            let resource_arg = cx.argument::<JsValue>(0)?;
            let resource: Resource = neon_serde::from_value(&mut cx, resource_arg)?;

            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut engine = this.borrow_mut(&guard);
                engine.resource_add(resource);
            }
            Ok(JsNull::new().upcast())
        }

        method getResource(mut cx) {
            let name: String = cx.argument::<JsString>(0)?.value();
            
            let this = cx.this();
            let result = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.resource_get(&name)
            };
            let js_value = neon_serde::to_value(&mut cx, &result)?;
            Ok(js_value)
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

fn validate_request(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let url: String = cx.argument::<JsString>(0)?.value();
    let source_url: String = cx.argument::<JsString>(1)?.value();
    let request_type: String = cx.argument::<JsString>(2)?.value();
    let request_ok = adblock::request::Request::from_urls(&url, &source_url, &request_type).is_ok();

    Ok(cx.boolean(request_ok))
}

fn ublock_resources(mut cx: FunctionContext) -> JsResult<JsValue> {
    let web_accessible_resource_dir: String = cx.argument::<JsString>(0)?.value();
    let redirect_engine_path: String = cx.argument::<JsString>(1)?.value();
    let scriptlets_path: String = cx.argument::<JsString>(2)?.value();

    let mut resources = assemble_web_accessible_resources(&Path::new(&web_accessible_resource_dir), &Path::new(&redirect_engine_path));
    resources.append(&mut assemble_scriptlet_resources(&Path::new(&scriptlets_path)));

    let js_resources = neon_serde::to_value(&mut cx, &resources)?;

    Ok(js_resources)
}

register_module!(mut m, {
    // Export the `JsEngine` class
    m.export_class::<JsEngine>("Engine")?;
    m.export_function("lists", lists)?;
    m.export_function("validateRequest", validate_request)?;
    m.export_function("uBlockResources", ublock_resources)?;
    Ok(())
});
