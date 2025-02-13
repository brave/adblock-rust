use adblock::{
    lists::{FilterSet, ParseOptions},
    request::Request,
    Engine,
};

fn main() {
    let rules = vec![String::from("||yandex.*/clck/$~ping")];

    let debug_info = true;
    let mut filter_set = FilterSet::new(debug_info);
    filter_set.add_filters(&rules, ParseOptions::default());

    let engine = Engine::from_filter_set(filter_set, true);

    let request = Request::new(
        "https://yandex.ru/clck/counter",
        "https://www.yandex.ru/",
        "other",
    )
    .unwrap();
    let blocker_result = engine.check_network_request(&request);

    println!("Blocker result: {:?}", blocker_result);
}
