#![allow(dead_code)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[cfg(test)] #[macro_use] extern crate serde;

extern crate regex;
extern crate punycode;
extern crate rayon;
extern crate idna;

pub mod utils;
pub mod request;

pub mod lists;
pub mod filters;

