use adblock::{
    lists::{FilterSet, ParseOptions},
    request::Request,
    Engine,
};

fn main() {
    let rules = [
        "-advertisement-icon.",
        "-advertisement-management/",
        "-advertisement.",
        "-advertisement/script.",
    ]
    .join("\n");

    let debug_info = true;
    let mut filter_set = FilterSet::new(debug_info);
    filter_set.add_filter_list(rules, ParseOptions::default());

    let engine = Engine::new_with_filter_set(filter_set, true);

    let request = Request::new(
        "http://example.com/-advertisement-icon.",
        "http://example.com/helloworld",
        "image",
    )
    .unwrap();
    let blocker_result = engine.check_network_request(&request);

    println!("Blocker result: {blocker_result:?}");
}
