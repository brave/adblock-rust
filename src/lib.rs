#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

extern crate regex;

pub mod utils;
pub mod request;

mod filters;
use filters::network;
