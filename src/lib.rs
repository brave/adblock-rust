#![allow(dead_code)]
#![forbid(unsafe_code)]

// Own modules, currently everything is exposed, will need to limit
#[doc(hidden)]
pub mod utils;
pub mod request;
pub mod lists;
pub mod filters;
pub mod blocker;
pub mod optimizer;
pub mod url_parser;
pub mod engine;
pub mod resources;
pub mod cosmetic_filter_cache;
pub mod data_format;
#[cfg(feature = "content-blocking")]
pub mod content_blocking;
