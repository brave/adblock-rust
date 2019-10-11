#![allow(dead_code)]
#![forbid(unsafe_code)]

// extern crate jemallocator;

// #[global_allocator]
// static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[macro_use]
extern crate lazy_static;

extern crate graphannis_malloc_size_of as malloc_size_of;
#[macro_use]
extern crate graphannis_malloc_size_of_derive as malloc_size_of_derive;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate matches; // #[cfg(test)]

extern crate rmp_serde as rmps;   // binary serialization/deserialization
extern crate flate2;
extern crate regex;
extern crate idna;      // utf domain handling
extern crate base64;
extern crate smallvec;

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
pub mod filter_lists;
pub mod resources;
