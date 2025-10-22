use adblock::{
    lists::{FilterSet, ParseOptions},
    request::Request,
    Engine,
};

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

    let engine = Engine::from_filter_set(filter_set, true);

    let request = Request::new(
        "http://example.com/-advertisement-icon.",
        "http://example.com/helloworld",
        "image",
    )
    .unwrap();
    let blocker_result = engine.check_network_request(&request);

    println!("Blocker result: {blocker_result:?}");
}
