#![no_main]
#![allow(unused_must_use)] // workaround for "error: unused `Result` that must be used"

//use adblock::lists::parse_filter;
use libfuzzer_sys::fuzz_target;
// use std::collections::HashSet;
use adblock::network_filter_list::NetworkFilterList;  // Adjust the path if needed based on your crate structure
use adblock::request::Request;
use adblock::regex_manager::RegexManager;
use std::collections::HashSet;
//use adblock_rust::network_filter_list::NetworkFilterList;

fuzz_target!(|data: &[u8]| {
  let r = NetworkFilterList::try_from_unverified_memory(data.to_vec());
  if let Ok(filter_list) = r {
    let mut regex_manager = RegexManager::default();
    let request = Request::new("https://example.com", "https://example.com", "script").unwrap();
    let matched_rule = filter_list.check(&request, &HashSet::new(), &mut regex_manager);
  }
});
