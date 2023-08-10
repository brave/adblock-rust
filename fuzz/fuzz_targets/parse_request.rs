#![no_main]
#![allow(unused_must_use)] // workaround for "error: unused `Result` that must be used"

use libfuzzer_sys::fuzz_target;
use adblock::request::Request;

fuzz_target!(|data: &[u8]| {
    if let Ok(url) = std::str::from_utf8(data) {
        Request::new(&format!("https://{}", url), "https://example.com", "other");
        Request::new(url, "https://example.com", "script");
        Request::new(url, "", "");
        Request::new(url, url, "");
        Request::new(url, url, url);
    }
});
