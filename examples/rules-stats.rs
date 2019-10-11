extern crate adblock;
extern crate jemallocator;

// #[global_allocator]
// static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

// use smallstring;
use adblock::engine::Engine;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::filters::network::NetworkFilter;
use adblock::lists::FilterList;
use graphannis_malloc_size_of::{MallocSizeOf, MallocSizeOfOps};
extern crate smallstring;
use smallstring::SmallString;

#[cfg(not(windows))]
pub mod platform {
    use std::os::raw::c_void;

    /// Defines which actual function is used.
    ///
    /// We always use the system malloc instead of jemalloc.
    /// On MacOS X, the external function is not called "malloc_usable_size", but "malloc_size"
    /// (it basically does the same).
    extern "C" {
        #[cfg_attr(any(target_os = "macos", target_os = "ios"), link_name = "malloc_size")]
        fn malloc_usable_size(ptr: *const c_void) -> usize;
    }

    /// Get the size of a heap block.
    pub unsafe extern "C" fn usable_size(ptr: *const c_void) -> usize {
        // jemallocator::usable_size(ptr)
        if ptr.is_null() {
            0
        } else {
            malloc_usable_size(ptr)
        }
    }
}


fn get_blocker_engine(filter_lists: &Vec<FilterList>) -> Engine {
    let network_filters: Vec<NetworkFilter> = filter_lists
        .iter()
        .map(|list| {
            let filters: Vec<String> = reqwest::get(&list.url)
                .expect("Could not request rules")
                .text()
                .expect("Could not get rules as text")
                .lines()
                .map(|s| s.to_owned())
                .collect();

            let (network_filters, _) = adblock::lists::parse_filters(&filters, true, false, true);
            network_filters
        })
        .flatten()
        .collect();

    let blocker_options = BlockerOptions {
        debug: false,
        enable_optimizations: true,
        load_cosmetic_filters: false,
        load_network_filters: true,
    };

    let mut engine = Engine {
        blocker: Blocker::new(network_filters, &blocker_options),
    };

    engine.with_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn average(numbers: &[i32]) -> f32 {
    numbers.iter().sum::<i32>() as f32 / numbers.len() as f32
}

fn percentile(numbers: &mut [i32], percentile: u32) -> i32 {
    numbers.sort();
    let mid = (numbers.len() as f32 * (percentile as f32)/100.0) as usize;
    numbers[mid]
}

fn print_stats(name: &str, mut items: &mut Vec<i32>) {
    println!("{}:\t\t Avg: {}  Med: {}, 95th: {}, Max: {:?}", name, average(items), percentile(&mut items, 50), percentile(&mut items, 95), items.iter().max());
}

fn main() {
    let network_filters: Vec<NetworkFilter> = adblock::filter_lists::default::default_lists()
    .iter()
    .map(|list| {
        let filters: Vec<String> = reqwest::get(&list.url)
            .expect("Could not request rules")
            .text()
            .expect("Could not get rules as text")
            .lines()
            .map(|s| s.to_owned())
            .collect();

        let (network_filters, _) = adblock::lists::parse_filters(&filters, true, false, true);
        network_filters
    })
    .flatten()
    .collect();

    
    let mut network_filter_opt_domains: Vec<_> = network_filters.iter().map(|f| f.opt_domains.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter opt domains", &mut network_filter_opt_domains);

    let mut network_filter_opt_not_domains: Vec<_> = network_filters.iter().map(|f| f.opt_not_domains.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter opt NOT domains", &mut network_filter_opt_not_domains);

    let mut hostname_lengths: Vec<_> = network_filters.iter().map(|f| f.hostname.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter hostname lengths", &mut hostname_lengths);

    let mut redirect_lengths: Vec<_> = network_filters.iter().map(|f| f.redirect.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter redirect lengths", &mut redirect_lengths);

    let mut csp_lengths: Vec<_> = network_filters.iter().map(|f| f.csp.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter csp lengths", &mut csp_lengths);

    let mut tag_lengths: Vec<_> = network_filters.iter().map(|f| f.tag.as_ref().map(|d| d.len() as i32).unwrap_or(0)).collect();
    print_stats("Network filter tag lengths", &mut tag_lengths);
}
