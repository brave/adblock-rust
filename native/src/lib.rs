extern crate neon;
extern crate neon_serde;
extern crate adblock;
extern crate serde;

use neon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use adblock::engine::Engine;
use adblock::lists::{RuleTypes, FilterFormat, FilterSet};
use adblock::resources::Resource;
use adblock::resources::resource_assembler::{assemble_web_accessible_resources, assemble_scriptlet_resources};


#[derive(Serialize, Deserialize)]
struct EngineOptions {
    pub optimize: Option<bool>,
}

declare_types! {
    pub class JsFilterSet for FilterSet {
        init(mut cx) {
            match cx.argument_opt(0) {
                Some(arg) => Ok(FilterSet::new(arg.downcast::<JsBoolean>().or_throw(&mut cx)?.value())),
                None => Ok(FilterSet::default()),
            }
        }

        method addFilters(mut cx) {
            // Take the first argument, which must be an array
            let rules_handle: Handle<JsArray> = cx.argument(0)?;
            // Second argument is the optional format of the rules, defaulting to
            // FilterFormat::Standard
            let format = match cx.argument_opt(1) {
                Some(format_arg) => neon_serde::from_value(&mut cx, format_arg)?,
                None => FilterFormat::Standard,
            };

            // Convert a JsArray to a Rust Vec
            let rules_wrapped: Vec<_> = rules_handle.to_vec(&mut cx)?;

            let mut rules: Vec<String> = vec![];
            for rule_wrapped in rules_wrapped {
                let rule = rule_wrapped.downcast::<JsString>().or_throw(&mut cx)?
                    .value();
                rules.push(rule);
            }

            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut filter_set = this.borrow_mut(&guard);
                filter_set.add_filters(&rules, format);
            }

            Ok(JsNull::new().upcast())
        }

        method addFilter(mut cx) {
            let filter: String = cx.argument::<JsString>(0)?.value();
            let format = match cx.argument_opt(1) {
                Some(format_arg) => neon_serde::from_value(&mut cx, format_arg)?,
                None => FilterFormat::Standard,
            };

            let mut this = cx.this();
            let guard = cx.lock();
            let ok = {
                let mut filter_set = this.borrow_mut(&guard);
                filter_set.add_filter(&filter, format).is_ok()
            };
            // Return true/false depending on whether or not the filter could be added
            Ok(JsBoolean::new(&mut cx, ok).upcast())
        }

        method intoContentBlocking(mut cx) {
            let rule_types = match cx.argument_opt(0) {
                Some(rule_types) => neon_serde::from_value(&mut cx, rule_types)?,
                None => RuleTypes::default(),
            };

            let this = cx.this();
            let guard = cx.lock();

            let cloned = {
                let filter_set = this.borrow(&guard);
                filter_set.clone()
            };

            match cloned.into_content_blocking(rule_types) {
                Ok((cb_rules, filters_used)) => {
                    let cb_rules = neon_serde::to_value(&mut cx, &cb_rules)?;
                    let filters_used = neon_serde::to_value(&mut cx, &filters_used)?;
                    let js_result = JsObject::new(&mut cx);
                    js_result.set(&mut cx, "contentBlockingRules", cb_rules)?;
                    js_result.set(&mut cx, "filtersUsed", filters_used)?;
                    Ok(js_result.upcast())
                }
                Err(_) => return Ok(JsUndefined::new().upcast()),
            }
        }
    }
}

declare_types! {
    pub class JsEngine for Engine {
        init(mut cx) {
            // Take the first argument, which must be a JsFilterSet
            let rules_handle: Handle<JsFilterSet> = cx.argument(0)?;
            let rules: FilterSet = {
                let guard = cx.lock();
                let rules = rules_handle.borrow(&guard);
                rules.to_owned()
            };

            match cx.argument_opt(1) {
                Some(arg) => {
                    // Throw if the argument exist and it cannot be downcasted to a boolean
                    let maybe_config: Result<EngineOptions, _> = neon_serde::from_value(&mut cx, arg);
                    let optimize = if let Ok(config) = maybe_config {
                        config.optimize.unwrap_or(true)
                    } else {
                        true
                    };
                    Ok(Engine::from_filter_set(rules, optimize))
                }
                None => {
                    Ok(Engine::from_filter_set(rules, true))
                },
            }
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
                engine.enable_tags(&[&tag])
            };
            Ok(JsNull::new().upcast())
        }

        method useResources(mut cx) {
            let resources_arg = cx.argument::<JsValue>(0)?;
            let resources: Vec<Resource> = neon_serde::from_value(&mut cx, resources_arg)?;

            let mut this = cx.this();
            let guard = cx.lock();
            {
                let mut engine = this.borrow_mut(&guard);
                engine.use_resources(&resources);
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
                // using an empty list of tags disables all tags
                engine.use_tags(&[]);
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
                engine.add_resource(resource);
            }
            Ok(JsNull::new().upcast())
        }

        method getResource(mut cx) {
            let name: String = cx.argument::<JsString>(0)?.value();
            
            let this = cx.this();
            let result = {
                let guard = cx.lock();
                let engine = this.borrow(&guard);
                engine.get_resource(&name)
            };
            let js_value = neon_serde::to_value(&mut cx, &result)?;
            Ok(js_value)
        }
    }
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

fn build_filter_format_enum<'a, C: Context<'a>>(cx: &mut C) -> JsResult<'a, JsObject> {
    let filter_format_enum = JsObject::new(cx);

    let standard = neon_serde::to_value(cx, &FilterFormat::Standard)?;
    filter_format_enum.set(cx, "STANDARD", standard)?;

    let hosts = neon_serde::to_value(cx, &FilterFormat::Hosts)?;
    filter_format_enum.set(cx, "HOSTS", hosts)?;

    Ok(filter_format_enum)
}

fn build_rule_types_enum<'a, C: Context<'a>>(cx: &mut C) -> JsResult<'a, JsObject> {
    let rule_types_enum = JsObject::new(cx);

    let all = neon_serde::to_value(cx, &RuleTypes::All)?;
    rule_types_enum.set(cx, "ALL", all)?;

    let network_only = neon_serde::to_value(cx, &RuleTypes::NetworkOnly)?;
    rule_types_enum.set(cx, "NETWORK_ONLY", network_only)?;

    let cosmetic_only = neon_serde::to_value(cx, &RuleTypes::CosmeticOnly)?;
    rule_types_enum.set(cx, "COSMETIC_ONLY", cosmetic_only)?;

    Ok(rule_types_enum)
}

register_module!(mut m, {
    m.export_class::<JsFilterSet>("FilterSet")?;
    m.export_class::<JsEngine>("Engine")?;

    m.export_function("validateRequest", validate_request)?;
    m.export_function("uBlockResources", ublock_resources)?;

    let filter_format_enum = build_filter_format_enum(&mut m)?;
    m.export_value("FilterFormat", filter_format_enum)?;

    let rule_types_enum = build_rule_types_enum(&mut m)?;
    m.export_value("RuleTypes", rule_types_enum)?;

    Ok(())
});
