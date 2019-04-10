#![allow(dead_code)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate matches; // #[cfg(test)]

extern crate bincode;   // binary serialization/deserialization
extern crate regex;
extern crate punycode;  // utf domain handling
extern crate idna;      // utf domain handling

#[cfg(target_arch = "wasm32")]
extern crate rayon;     // parallelism

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
