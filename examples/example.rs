use adblock::engine::Engine;
use adblock::lists::{FilterSet, ParseOptions};

fn main() {
    let rules = vec![
        String::from("-advertisement-icon."),
        String::from("-advertisement-management/"),
        String::from("-advertisement."),
        String::from("-advertisement/script."),
    ];

    let debug_info = true;
    let mut filter_set = FilterSet::new(debug_info);
    filter_set.add_filters(&rules, ParseOptions::default());

    let blocker = Engine::from_filter_set(filter_set, true);
    let blocker_result = blocker.check_network_urls(
        "http://example.com/-advertisement-icon.",
        "http://example.com/helloworld",
        "image",
    );

    println!("Blocker result: {:?}", blocker_result);
}
