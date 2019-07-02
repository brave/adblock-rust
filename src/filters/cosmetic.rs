//! Tools for blocking at a page-content level, including CSS selector-based filtering and content
//! script injection.
use serde::{Deserialize, Serialize};
use crate::utils::Hash;

bitflags! {
    /// Boolean flags for cosmetic filter rules.
    #[derive(Serialize, Deserialize)]
    pub struct CosmeticFilterMask: u8 {
        const UNHIDE = 1 << 0;
        const SCRIPT_INJECT = 1 << 1;
        const IS_UNICODE = 1 << 2;
        const IS_CLASS_SELECTOR = 1 << 3;
        const IS_ID_SELECTOR = 1 << 4;
        const IS_HREF_SELECTOR = 1 << 5;

        // Careful with checking for NONE - will always match
        const NONE = 0;
    }
}

/// Struct representing a parsed cosmetic filter rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticFilter {
    pub entities: Option<Vec<Hash>>,
    pub hostnames: Option<Vec<Hash>>,
    pub mask: CosmeticFilterMask,
    pub not_entities: Option<Vec<Hash>>,
    pub not_hostnames: Option<Vec<Hash>>,
    pub raw_line: Option<String>,
    pub selector: String,
    pub style: Option<String>,
}

impl CosmeticFilter {
    /// Parse the rule in `line` into a `CosmeticFilter`. If `debug` is true, the original rule
    /// will be reported in the resulting `CosmeticFilter` struct as well.
    pub fn parse(line: &str, _debug: bool) -> Result<CosmeticFilter, crate::filters::network::FilterError> {
        // TODO: unimplemented, just return rule as a string
        Ok(CosmeticFilter {
            entities: None,
            hostnames: None,
            mask: CosmeticFilterMask::NONE,
            not_entities: None,
            not_hostnames: None,
            raw_line: Some(String::from(line)),
            selector: String::from(line),
            style: None,
        })
    }
}
