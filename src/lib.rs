#![allow(dead_code)]
#![forbid(unsafe_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[macro_use]
#[cfg(test)]
extern crate matches;

extern crate regex;
extern crate punycode;
extern crate rayon;
extern crate idna;

pub mod utils;
pub mod request;

pub mod lists;
pub mod filters;
pub mod blocker;

pub mod optimizer;
