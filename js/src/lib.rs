use neon::prelude::*;
use neon::types::buffer::TypedArray as _;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::sync::Mutex;
use std::path::Path;
use adblock::Engine as EngineInternal;
use adblock::EngineSerializer as EngineSerializerInternal;
use adblock::lists::{RuleTypes, FilterFormat, FilterListMetadata, FilterSet as FilterSetInternal, ParseOptions};
use adblock::resources::Resource;
use adblock::resources::resource_assembler::assemble_web_accessible_resources;

/// Use the JS context's JSON.stringify and JSON.parse as an FFI, at least until
/// https://github.com/neon-bindings/neon/pull/953 is available
mod json_ffi {
    use super::*;
    use serde::de::DeserializeOwned;

    /// Call `JSON.stringify` to convert the input to a `JsString`, then call serde_json to parse
    /// it to an instance of a native Rust type
    pub fn from_js<'a, C: Context<'a>, T: DeserializeOwned>(cx: &mut C, input: Handle<JsValue>) -> NeonResult<T> {
        let json: Handle<JsObject> = cx.global().get(cx, "JSON")?;
        let json_stringify: Handle<JsFunction> = json.get(cx, "stringify")?;

        let undefined = JsUndefined::new(cx);
        let js_string = json_stringify
            .call(cx, undefined, [input])?
            .downcast::<JsString, _>(cx).or_throw(cx)?;

        match serde_json::from_str(&js_string.value(cx)) {
            Ok(v) => Ok(v),
            Err(e) => cx.throw_error(e.to_string())?,
        }
    }

    /// Use `serde_json` to stringify the input, then call `JSON.parse` to convert it to a
    /// `JsValue`
    pub fn to_js<'a, C: Context<'a>, T: serde::Serialize>(cx: &mut C, input: &T) -> JsResult<'a, JsValue> {
        let input_handle = JsString::new(cx, serde_json::to_string(&input).unwrap());

        let json: Handle<JsObject> = cx.global().get(cx, "JSON")?;
        let json_parse: Handle<JsFunction> = json.get(cx, "parse")?;

        json_parse
            .call_with(cx)
            .arg(input_handle)
            .apply(cx)
    }
}

#[derive(Serialize, Deserialize)]
struct EngineOptions {
    pub optimize: Option<bool>,
}

#[derive(Default)]
struct FilterSet(RefCell<FilterSetInternal>);
impl FilterSet {
    fn new(debug: bool) -> Self {
        Self(RefCell::new(FilterSetInternal::new(debug)))
    }
    fn add_filters(&self, rules: &[String], opts: ParseOptions) -> FilterListMetadata {
        self.0.borrow_mut().add_filters(rules, opts)
    }
    fn add_filter(&self, filter: &str, opts: ParseOptions) -> Result<(), adblock::lists::FilterParseError> {
        self.0.borrow_mut().add_filter(filter, opts)
    }
    fn into_content_blocking(&self) -> Result<(Vec<adblock::content_blocking::CbRule>, Vec<String>), ()> {
        self.0.borrow().clone().into_content_blocking()
    }
}

impl Finalize for FilterSet {}

fn create_filter_set(mut cx: FunctionContext) -> JsResult<JsBox<FilterSet>> {
    match cx.argument_opt(0) {
        Some(arg) => {
            let debug: bool = arg.downcast::<JsBoolean, _>(&mut cx).or_throw(&mut cx)?.value(&mut cx);
            Ok(cx.boxed(FilterSet::new(debug)))
        }
        None => Ok(cx.boxed(FilterSet::default())),
    }
}

fn filter_set_add_filters(mut cx: FunctionContext) -> JsResult<JsValue> {
    let this = cx.argument::<JsBox<FilterSet>>(0)?;

    // Take the first argument, which must be an array
    let rules_handle: Handle<JsValue> = cx.argument(1)?;
    // Second argument is optional parse options. All fields are optional. ParseOptions::default()
    // if unspecified.
    let parse_opts = match cx.argument_opt(2) {
        Some(parse_opts_arg) => json_ffi::from_js(&mut cx, parse_opts_arg)?,
        None => ParseOptions::default(),
    };

    let rules: Vec<String> = json_ffi::from_js(&mut cx, rules_handle)?;

    let metadata = this.add_filters(&rules, parse_opts);

    json_ffi::to_js(&mut cx, &metadata)
}

fn filter_set_add_filter(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let this = cx.argument::<JsBox<FilterSet>>(0)?;

    let filter: String = cx.argument::<JsString>(1)?.value(&mut cx);
    let parse_opts = match cx.argument_opt(2) {
        Some(parse_opts_arg) => json_ffi::from_js(&mut cx, parse_opts_arg)?,
        None => ParseOptions::default(),
    };

    let ok = this.add_filter(&filter, parse_opts).is_ok();
    // Return true/false depending on whether or not the filter could be added
    Ok(JsBoolean::new(&mut cx, ok))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentBlockingConversionResult {
    content_blocking_rules: Vec<adblock::content_blocking::CbRule>,
    filters_used: Vec<String>,
}

fn filter_set_into_content_blocking(mut cx: FunctionContext) -> JsResult<JsValue> {
    let this = cx.argument::<JsBox<FilterSet>>(0)?;

    match this.into_content_blocking() {
        Ok((cb_rules, filters_used)) => {
            let r = ContentBlockingConversionResult {
                content_blocking_rules: cb_rules,
                filters_used,
            };
            json_ffi::to_js(&mut cx, &r)
        }
        Err(_) => return Ok(JsUndefined::new(&mut cx).upcast()),
    }
}

struct Engine(Mutex<EngineInternal>);

impl Finalize for Engine {}

unsafe impl Send for Engine {}

fn engine_constructor(mut cx: FunctionContext) -> JsResult<JsBox<Engine>> {
    // Take the first argument, which must be a JsFilterSet
    let rules = cx.argument::<JsBox<FilterSet>>(0)?;
    let rules = rules.0.borrow().clone();

    let engine_internal = match cx.argument_opt(1) {
        Some(arg) => {
            let optimize = match arg.downcast::<JsBoolean, _>(&mut cx) {
                Ok(b) => b.value(&mut cx),
                Err(_) => {
                    let config = json_ffi::from_js::<_, EngineOptions>(&mut cx, arg)?;
                    config.optimize.unwrap_or(true)
                }
            };
            EngineInternal::from_filter_set(rules, optimize)
        }
        None => {
            EngineInternal::from_filter_set(rules, true)
        },
    };
    Ok(cx.boxed(Engine(Mutex::new(engine_internal))))
}

fn engine_check(mut cx: FunctionContext) -> JsResult<JsValue> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let url: String = cx.argument::<JsString>(1)?.value(&mut cx);
    let source_url: String = cx.argument::<JsString>(2)?.value(&mut cx);
    let request_type: String = cx.argument::<JsString>(3)?.value(&mut cx);

    let debug = match cx.argument_opt(4) {
        Some(arg) => {
            // Throw if the argument exists and it cannot be downcasted to a boolean
            arg.downcast::<JsBoolean, _>(&mut cx).or_throw(&mut cx)?.value(&mut cx)
        }
        None => false,
    };

    let request = match adblock::request::Request::new(&url, &source_url, &request_type) {
        Ok(r) => r,
        Err(e) => cx.throw_error(e.to_string())?,
    };

    let result = if let Ok(engine) = this.0.lock() {
        engine.check_network_request(&request)
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    if debug {
        json_ffi::to_js(&mut cx, &result)
    } else {
        Ok(cx.boolean(result.matched).upcast())
    }
}

fn engine_hidden_class_id_selectors(mut cx: FunctionContext) -> JsResult<JsValue> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let classes_arg = cx.argument::<JsValue>(1)?;
    let classes: Vec<String> = json_ffi::from_js(&mut cx, classes_arg)?;

    let ids_arg = cx.argument::<JsValue>(2)?;
    let ids: Vec<String> = json_ffi::from_js(&mut cx, ids_arg)?;

    let exceptions_arg = cx.argument::<JsValue>(3)?;
    let exceptions: std::collections::HashSet<String> = json_ffi::from_js(&mut cx, exceptions_arg)?;

    let result = if let Ok(engine) = this.0.lock() {
        engine.hidden_class_id_selectors(&classes, &ids, &exceptions)
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    json_ffi::to_js(&mut cx, &result)
}

fn engine_url_cosmetic_resources(mut cx: FunctionContext) -> JsResult<JsValue> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let url: String = cx.argument::<JsString>(1)?.value(&mut cx);

    let result = if let Ok(engine) = this.0.lock() {
        engine.url_cosmetic_resources(&url)
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    json_ffi::to_js(&mut cx, &result)
}

fn engine_serialize_raw(mut cx: FunctionContext) -> JsResult<JsArrayBuffer> {
    let this = cx.argument::<JsBox<Engine>>(0)?;
    let serialized = if let Ok(engine) = this.0.lock() {
        engine.serialize_raw().unwrap()
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };

    // initialise new Array Buffer in the JS context
    let mut buffer = JsArrayBuffer::new(&mut cx, serialized.len())?;
    // copy data from Rust buffer to JS Array Buffer
    buffer.as_mut_slice(&mut cx).copy_from_slice(&serialized);

    Ok(buffer)
}

fn engine_deserialize(mut cx: FunctionContext) -> JsResult<JsNull> {
    let this = cx.argument::<JsBox<Engine>>(0)?;
    let serialized_handle = cx.argument::<JsArrayBuffer>(1)?;

    if let Ok(mut engine) = this.0.lock() {
        let _result = engine.deserialize(&serialized_handle.as_slice(&mut cx));
    }

    Ok(JsNull::new(&mut cx))
}

fn engine_enable_tag(mut cx: FunctionContext) -> JsResult<JsNull> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let tag: String = cx.argument::<JsString>(1)?.value(&mut cx);

    if let Ok(mut engine) = this.0.lock() {
        engine.enable_tags(&[&tag])
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    Ok(JsNull::new(&mut cx))
}

fn engine_use_resources(mut cx: FunctionContext) -> JsResult<JsNull> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let resources_arg = cx.argument::<JsValue>(1)?;
    let resources: Vec<Resource> = json_ffi::from_js(&mut cx, resources_arg)?;

    if let Ok(mut engine) = this.0.lock() {
        engine.use_resources(resources)
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    Ok(JsNull::new(&mut cx))
}

fn engine_tag_exists(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let tag: String = cx.argument::<JsString>(1)?.value(&mut cx);

    let result = if let Ok(engine) = this.0.lock() {
        engine.tag_exists(&tag)
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    Ok(cx.boolean(result))
}

fn engine_clear_tags(mut cx: FunctionContext) -> JsResult<JsNull> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    if let Ok(mut engine) = this.0.lock() {
        engine.use_tags(&[]);
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    Ok(JsNull::new(&mut cx))
}

fn engine_add_resource(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let this = cx.argument::<JsBox<Engine>>(0)?;

    let resource_arg = cx.argument::<JsValue>(1)?;
    let resource: Resource = json_ffi::from_js(&mut cx, resource_arg)?;

    let success = if let Ok(mut engine) = this.0.lock() {
        engine.add_resource(resource).is_ok()
    } else {
        cx.throw_error("Failed to acquire lock on engine")?
    };
    Ok(cx.boolean(success))
}

fn validate_request(mut cx: FunctionContext) -> JsResult<JsBoolean> {
    let url: String = cx.argument::<JsString>(0)?.value(&mut cx);
    let source_url: String = cx.argument::<JsString>(1)?.value(&mut cx);
    let request_type: String = cx.argument::<JsString>(2)?.value(&mut cx);
    let request_ok = adblock::request::Request::new(&url, &source_url, &request_type).is_ok();

    Ok(cx.boolean(request_ok))
}

fn ublock_resources(mut cx: FunctionContext) -> JsResult<JsValue> {
    let web_accessible_resource_dir: String = cx.argument::<JsString>(0)?.value(&mut cx);
    let redirect_resources_path: String = cx.argument::<JsString>(1)?.value(&mut cx);
    // `scriptlets_path` is optional, since adblock-rust parsing that file is now deprecated.
    let scriptlets_path = match cx.argument_opt(2) {
        Some(arg) => Some(arg.downcast::<JsString, _>(&mut cx).or_throw(&mut cx)?.value(&mut cx)),
        None => None,
    };

    let mut resources = assemble_web_accessible_resources(&Path::new(&web_accessible_resource_dir), &Path::new(&redirect_resources_path));
    if let Some(scriptlets_path) = scriptlets_path {
        #[allow(deprecated)]
        resources.append(&mut adblock::resources::resource_assembler::assemble_scriptlet_resources(&Path::new(&scriptlets_path)));
    }

    json_ffi::to_js(&mut cx, &resources)
}

fn build_filter_format_enum<'a, C: Context<'a>>(cx: &mut C) -> JsResult<'a, JsObject> {
    let filter_format_enum = JsObject::new(cx);

    let standard = json_ffi::to_js(cx, &FilterFormat::Standard)?;
    filter_format_enum.set(cx, "STANDARD", standard)?;

    let hosts = json_ffi::to_js(cx, &FilterFormat::Hosts)?;
    filter_format_enum.set(cx, "HOSTS", hosts)?;

    Ok(filter_format_enum)
}

fn build_rule_types_enum<'a, C: Context<'a>>(cx: &mut C) -> JsResult<'a, JsObject> {
    let rule_types_enum = JsObject::new(cx);

    let all = json_ffi::to_js(cx, &RuleTypes::All)?;
    rule_types_enum.set(cx, "ALL", all)?;

    let network_only = json_ffi::to_js(cx, &RuleTypes::NetworkOnly)?;
    rule_types_enum.set(cx, "NETWORK_ONLY", network_only)?;

    let cosmetic_only = json_ffi::to_js(cx, &RuleTypes::CosmeticOnly)?;
    rule_types_enum.set(cx, "COSMETIC_ONLY", cosmetic_only)?;

    Ok(rule_types_enum)
}

register_module!(mut m, {
    m.export_function("FilterSet_constructor", create_filter_set)?;
    m.export_function("FilterSet_addFilters", filter_set_add_filters)?;
    m.export_function("FilterSet_addFilter", filter_set_add_filter)?;
    m.export_function("FilterSet_intoContentBlocking", filter_set_into_content_blocking)?;

    m.export_function("Engine_constructor", engine_constructor)?;
    m.export_function("Engine_check", engine_check)?;
    m.export_function("Engine_urlCosmeticResources", engine_url_cosmetic_resources)?;
    m.export_function("Engine_hiddenClassIdSelectors", engine_hidden_class_id_selectors)?;
    m.export_function("Engine_serializeRaw", engine_serialize_raw)?;
    m.export_function("Engine_deserialize", engine_deserialize)?;
    m.export_function("Engine_enableTag", engine_enable_tag)?;
    m.export_function("Engine_useResources", engine_use_resources)?;
    m.export_function("Engine_tagExists", engine_tag_exists)?;
    m.export_function("Engine_clearTags", engine_clear_tags)?;
    m.export_function("Engine_addResource", engine_add_resource)?;

    m.export_function("validateRequest", validate_request)?;
    m.export_function("uBlockResources", ublock_resources)?;

    let filter_format_enum = build_filter_format_enum(&mut m)?;
    m.export_value("FilterFormat", filter_format_enum)?;

    let rule_types_enum = build_rule_types_enum(&mut m)?;
    m.export_value("RuleTypes", rule_types_enum)?;

    Ok(())
});
