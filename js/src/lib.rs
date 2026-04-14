use adblock::lists::{
    FilterFormat, FilterListMetadata, FilterSet as FilterSetInternal, ParseOptions, RuleTypes,
};
use adblock::resources::resource_assembler::assemble_web_accessible_resources;
use adblock::resources::Resource;
use adblock::Engine as EngineInternal;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Serialize;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// FilterSet
// ---------------------------------------------------------------------------

#[napi(js_name = "FilterSet")]
pub struct JsFilterSet {
    inner: RefCell<FilterSetInternal>,
}

// Safety: the `single-thread` feature on adblock makes FilterSetInternal !Send,
// but NAPI-RS classes require Send. The JS runtime is single-threaded so this is
// safe in practice.
unsafe impl Send for JsFilterSet {}

#[napi]
impl JsFilterSet {
    #[napi(constructor)]
    pub fn new(debug: Option<bool>) -> Self {
        Self {
            inner: RefCell::new(FilterSetInternal::new(debug.unwrap_or(false))),
        }
    }

    #[napi]
    pub fn add_filters(
        &self,
        rules: Vec<String>,
        opts: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let parse_opts: ParseOptions = match opts {
            Some(v) => serde_json::from_value(v).map_err(|e| Error::from_reason(e.to_string()))?,
            None => ParseOptions::default(),
        };
        let metadata: FilterListMetadata = self.inner.borrow_mut().add_filters(&rules, parse_opts);
        serde_json::to_value(&metadata).map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn add_filter(&self, filter: String, opts: Option<serde_json::Value>) -> Result<bool> {
        let parse_opts: ParseOptions = match opts {
            Some(v) => serde_json::from_value(v).map_err(|e| Error::from_reason(e.to_string()))?,
            None => ParseOptions::default(),
        };
        Ok(self
            .inner
            .borrow_mut()
            .add_filter(&filter, parse_opts)
            .is_ok())
    }

    #[napi]
    pub fn into_content_blocking(&self) -> Result<Either<serde_json::Value, Undefined>> {
        match self.inner.borrow().clone().into_content_blocking() {
            Ok((cb_rules, filters_used)) => {
                let result = ContentBlockingConversionResult {
                    content_blocking_rules: cb_rules,
                    filters_used,
                };
                let val =
                    serde_json::to_value(&result).map_err(|e| Error::from_reason(e.to_string()))?;
                Ok(Either::A(val))
            }
            Err(_) => Ok(Either::B(())),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContentBlockingConversionResult {
    content_blocking_rules: Vec<adblock::content_blocking::CbRule>,
    filters_used: Vec<String>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

#[napi(js_name = "Engine")]
pub struct JsEngine {
    inner: Mutex<EngineInternal>,
}

// Safety: same rationale as JsFilterSet — single-threaded JS runtime.
unsafe impl Send for JsEngine {}

#[napi]
impl JsEngine {
    #[napi(constructor)]
    pub fn new(filter_set: &JsFilterSet, options: Option<serde_json::Value>) -> Self {
        let rules = filter_set.inner.borrow().clone();

        let optimize = match options {
            Some(serde_json::Value::Bool(b)) => b,
            Some(ref obj) => obj
                .get("optimize")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            None => true,
        };

        Self {
            inner: Mutex::new(EngineInternal::from_filter_set(rules, optimize)),
        }
    }

    #[napi]
    pub fn check(
        &self,
        url: String,
        source_url: String,
        request_type: String,
        debug: Option<bool>,
    ) -> Result<serde_json::Value> {
        let debug = debug.unwrap_or(false);
        let request = adblock::request::Request::new(&url, &source_url, &request_type)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        let result = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .check_network_request(&request);

        if debug {
            serde_json::to_value(&result).map_err(|e| Error::from_reason(e.to_string()))
        } else {
            Ok(serde_json::Value::Bool(result.matched))
        }
    }

    #[napi]
    pub fn url_cosmetic_resources(&self, url: String) -> Result<serde_json::Value> {
        let result = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .url_cosmetic_resources(&url);
        serde_json::to_value(&result).map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub fn hidden_class_id_selectors(
        &self,
        classes: Vec<String>,
        ids: Vec<String>,
        exceptions: Vec<String>,
    ) -> Result<Vec<String>> {
        let exceptions_set: std::collections::HashSet<String> = exceptions.into_iter().collect();
        let result = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .hidden_class_id_selectors(&classes, &ids, &exceptions_set);
        Ok(result)
    }

    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let serialized = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .serialize()
            .to_vec();
        Ok(serialized.into())
    }

    #[napi]
    pub fn deserialize(&self, buffer: Buffer) -> Result<()> {
        let mut engine = self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        let _ = engine.deserialize(&buffer);
        Ok(())
    }

    #[napi]
    pub fn enable_tag(&self, tag: String) -> Result<()> {
        self.inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .enable_tags(&[&tag]);
        Ok(())
    }

    #[napi]
    pub fn use_resources(&self, resources: serde_json::Value) -> Result<()> {
        let resources: Vec<Resource> =
            serde_json::from_value(resources).map_err(|e| Error::from_reason(e.to_string()))?;
        self.inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .use_resources(resources);
        Ok(())
    }

    #[napi]
    pub fn tag_exists(&self, tag: String) -> Result<bool> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .tag_exists(&tag))
    }

    #[napi]
    pub fn clear_tags(&self) -> Result<()> {
        self.inner
            .lock()
            .map_err(|e| Error::from_reason(e.to_string()))?
            .use_tags(&[]);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Standalone functions
// ---------------------------------------------------------------------------

#[napi]
pub fn validate_request(url: String, source_url: String, request_type: String) -> bool {
    adblock::request::Request::new(&url, &source_url, &request_type).is_ok()
}

#[napi(js_name = "FilterFormat")]
pub fn filter_format_enum() -> serde_json::Value {
    serde_json::json!({
        "STANDARD": serde_json::to_value(FilterFormat::Standard).unwrap(),
        "HOSTS": serde_json::to_value(FilterFormat::Hosts).unwrap(),
    })
}

#[napi(js_name = "RuleTypes")]
pub fn rule_types_enum() -> serde_json::Value {
    serde_json::json!({
        "ALL": serde_json::to_value(RuleTypes::All).unwrap(),
        "NETWORK_ONLY": serde_json::to_value(RuleTypes::NetworkOnly).unwrap(),
        "COSMETIC_ONLY": serde_json::to_value(RuleTypes::CosmeticOnly).unwrap(),
    })
}

#[napi(js_name = "uBlockResources")]
pub fn u_block_resources(
    web_accessible_resource_dir: String,
    redirect_resources_path: String,
    scriptlets_path: Option<String>,
) -> Result<serde_json::Value> {
    let mut resources = assemble_web_accessible_resources(
        Path::new(&web_accessible_resource_dir),
        Path::new(&redirect_resources_path),
    );
    if let Some(scriptlets_path) = scriptlets_path {
        #[allow(deprecated)]
        resources.extend(
            adblock::resources::resource_assembler::assemble_scriptlet_resources(Path::new(
                &scriptlets_path,
            )),
        );
    }
    serde_json::to_value(&resources).map_err(|e| Error::from_reason(e.to_string()))
}
