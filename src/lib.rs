#![allow(dead_code)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate matches; // #[cfg(test)]

extern crate rmp_serde as rmps;   // binary serialization/deserialization
extern crate flate2;
extern crate regex;
extern crate idna;      // utf domain handling
extern crate base64;

#[cfg(test)]
extern crate csv;       // csv handling library used for processing test data

// Own modules, currently everything is exposed, will need to limit
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
