#![allow(dead_code)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

extern crate regex;
extern crate punycode;
extern crate rayon;

pub mod utils;
pub mod request;

pub mod lists;
mod filters;

