#![no_main]
use libfuzzer_sys::fuzz_target;
use adblock::request::Request;

fuzz_target!(|data: &[u8]| {
    if let Ok(url) = std::str::from_utf8(data) {
        Request::from_url(&format!("https://{}", url));
        Request::from_urls(url, "https://example.com", "script");
        Request::from_urls(url, "", "");
        Request::from_urls(url, url, "");
        Request::from_urls(url, url, url);
    }
});
