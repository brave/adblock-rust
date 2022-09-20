#![no_main]
#![allow(unused_must_use)] // workaround for "error: unused `Result` that must be used"

use libfuzzer_sys::fuzz_target;
use adblock::lists::{parse_filter, FilterFormat, ParseOptions};

fuzz_target!(|data: &[u8]| {
    if let Ok(filter) = std::str::from_utf8(data) {
        parse_filter(filter, true, ParseOptions { format: FilterFormat::Hosts, ..Default::default() });
    }
});
