use adblock::request::Request;
use adblock::Engine;

#[allow(unused_imports)]
use std::collections::HashMap;

mod test_utils;
use test_utils::rules_from_lists;

#[cfg(not(debug_assertions))]
#[allow(non_snake_case)]
#[derive(Debug, serde::Deserialize)]
struct RequestRuleMatch {
    url: String,
    sourceUrl: String,
    r#type: String,
    blocked: u8,
    filter: Option<String>,
}

#[cfg(not(debug_assertions))]
fn load_requests() -> Vec<RequestRuleMatch> {
    use std::fs::File;
    use std::io::BufReader;

    let f = File::open("data/ublock-matches.tsv").expect("file not found");
    let reader = BufReader::new(f);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

    let mut reqs: Vec<RequestRuleMatch> = Vec::new();
    for result in rdr.deserialize() {
        if result.is_ok() {
            let record: RequestRuleMatch = result.unwrap();
            reqs.push(record);
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

fn get_blocker_engine() -> Engine {
    let rules = rules_from_lists([
        "data/regression-testing/easylist.txt",
        "data/regression-testing/easyprivacy.txt",
    ]);

    Engine::from_rules_parametrised(rules, Default::default(), true, false)
}

fn get_blocker_engine_default(extra_rules: impl IntoIterator<Item = impl AsRef<str>>) -> Engine {
    let rules = rules_from_lists([
        "data/easylist.to/easylist/easylist.txt",
        "data/easylist.to/easylist/easyprivacy.txt",
        "data/uBlockOrigin/unbreak.txt",
        "data/uBlockOrigin/filters.txt",
        "data/brave/brave-unbreak.txt",
        "data/brave/coin-miners.txt",
        // "data/test/abpjf.txt",
    ]);

    let all_rules = rules.chain(extra_rules.into_iter().map(|r| r.as_ref().to_string()));

    Engine::from_rules_parametrised(all_rules, Default::default(), true, false)
}

#[test]
fn check_specific_rules() {
    {
        // exceptions have not effect if important filter matches
        let engine = Engine::from_rules_debug(["||www.facebook.com/*/plugin"], Default::default());

        let request =
            Request::new("https://www.facebook.com/v3.2/plugins/comments.ph", "", "").unwrap();
        let checked = engine.check_network_request(&request);

        assert_eq!(checked.matched, true);
    }

    #[cfg(feature = "resource-assembler")]
    {
        use std::path::Path;

        // exceptions have no effect if important filter matches
        let mut engine = Engine::from_rules_debug(
            [
                "||cdn.taboola.com/libtrc/*/loader.js$script,redirect=noopjs,important,domain=cnet.com",
            ],
            Default::default(),
        );
        let resources = adblock::resources::resource_assembler::assemble_web_accessible_resources(
            Path::new("data/test/fake-uBO-files/web_accessible_resources"),
            Path::new("data/test/fake-uBO-files/redirect-resources.js"),
        );
        engine.use_resources(resources);

        let request = Request::new(
            "http://cdn.taboola.com/libtrc/test/loader.js",
            "http://cnet.com",
            "script",
        )
        .unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, true);
        assert_eq!(checked.redirect, Some("data:application/javascript;base64,KGZ1bmN0aW9uKCkgewogICAgJ3VzZSBzdHJpY3QnOwp9KSgpOwo=".to_owned()));
    }
}

#[test]
fn check_specifics_default() {
    let mut engine = get_blocker_engine_default([
        "@@||www.google.*/aclk?$first-party",
        "@@||www.googleadservices.*/aclk?$first-party",
    ]);
    {
        let request = Request::new("https://www.youtube.com/youtubei/v1/log_event?alt=json&key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8", "", "").unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, true);
    }
    {
        let request = Request::new(
            "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=",
            "https://www.google.com/aclk?sa=l&ai=DChcSEwioqMfq5ovjAhVvte0KHXBYDKoYABAJGgJkZw&sig=AOD64_0IL5OYOIkZA7qWOBt0yRmKL4hKJw&ctype=5&q=&ved=0ahUKEwjQ88Hq5ovjAhXYiVwKHWAgB5gQww8IXg&adurl=",
            "main_frame",
        ).unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, false, "Matched on {:?}", checked.filter);
    }
    {
        let request = Request::new(
            "https://www.googleadservices.com/pagead/aclk?sa=L&ai=DChcSEwin96uLgYzjAhWH43cKHf0JA7YYABABGgJlZg&ohost=www.google.com&cid=CAASEuRoSkQKbbu2CAjK-zZJnF-wcw&sig=AOD64_1j63JqPtw22vaMasSE4aN1FRKtEw&ctype=5&q=&ved=0ahUKEwivnaWLgYzjAhUERxUIHWzYDTQQ9A4IzgI&adurl=",
            "https://www.googleadservices.com/pagead/aclk?sa=L&ai=DChcSEwin96uLgYzjAhWH43cKHf0JA7YYABABGgJlZg&ohost=www.google.com&cid=CAASEuRoSkQKbbu2CAjK-zZJnF-wcw&sig=AOD64_1j63JqPtw22vaMasSE4aN1FRKtEw&ctype=5&q=&ved=0ahUKEwivnaWLgYzjAhUERxUIHWzYDTQQ9A4IzgI&adurl=",
            "main_frame",
        ).unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, false, "Matched on {:?}", checked.filter);
    }
    {
        let request = Request::new(
            "https://www.researchgate.net/profile/Ruofei_Zhang/publication/221653522_Bid_landscape_forecasting_in_online_Ad_exchange_marketplace/links/53f10c1f0cf2711e0c432641.pdf",
            "https://www.researchgate.net/profile/Ruofei_Zhang/publication/221653522_Bid_landscape_forecasting_in_online_Ad_exchange_marketplace/links/53f10c1f0cf2711e0c432641.pdf",
            "main_frame",
        ).unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, false, "Matched on {:?}", checked.filter);
    }
    {
        let request = Request::new(
            "https://www.google.com/search?q=Bid+Landscape+Forecasting+in+Online+Exchange+Marketplace&oq=Landscape+Forecasting+in+Online+Ad+Exchange+Marketplace",
            "https://www.google.com/search?q=Bid+Landscape+Forecasting+in+Online+Exchange+Marketplace&oq=Landscape+Forecasting+in+Online+Ad+Exchange+Marketplace",
            "main_frame",
        ).unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, false, "Matched on {:?}", checked.filter);
    }
    {
        engine.use_tags(&["fb-embeds", "twitter-embeds"]);
        let request = Request::new(
            "https://platform.twitter.com/widgets.js",
            "https://fmarier.github.io/brave-testing/social-widgets.html",
            "script",
        )
        .unwrap();
        let checked = engine.check_network_request(&request);
        assert!(checked.exception.is_some(), "Expected exception to match");
        assert!(checked.filter.is_some(), "Expected rule to match");
        assert_eq!(checked.matched, false, "Matched on {:?}", checked.exception)
    }
}

#[test]
fn check_basic_works_after_deserialization() {
    let engine = get_blocker_engine();
    let serialized = engine.serialize().unwrap();
    let mut deserialized_engine = Engine::default();
    deserialized_engine.deserialize(&serialized).unwrap();

    {
        let request = Request::new("https://www.youtube.com/youtubei/v1/log_event?alt=json&key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8", "", "").unwrap();
        let checked = engine.check_network_request(&request);
        assert_eq!(checked.matched, true);
    }
}

#[test]
#[cfg(not(debug_assertions))] // This test takes > 3 minutes in debug mode
fn check_matching_equivalent() {
    let requests = load_requests();

    assert!(requests.len() > 0, "List of parsed request info is empty");

    let engine = get_blocker_engine();

    let requests_len = requests.len() as u32;

    let mut mismatch_expected_match = 0;
    let mut mismatch_expected_exception = 0;
    let mut mismatch_expected_pass = 0;
    let mut false_negative_rules: HashMap<String, (String, String, String)> = HashMap::new();
    let mut false_positive_rules: HashMap<String, (String, String, String)> = HashMap::new();
    let mut false_negative_exceptions: HashMap<String, (String, String, String)> = HashMap::new();
    for req in requests {
        let request = Request::new(&req.url, &req.sourceUrl, &req.r#type).unwrap();
        let checked = engine.check_network_request(&request);
        if req.blocked == 1 && checked.matched != true {
            mismatch_expected_match += 1;
            req.filter.as_ref().map(|f| {
                false_negative_rules.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            println!(
                "Expected match, uBo matched {} at {}, type {} ON {:?}",
                req.url, req.sourceUrl, req.r#type, req.filter
            );
        } else if req.blocked == 2 && checked.exception.is_none() {
            mismatch_expected_exception += 1;
            checked.filter.as_ref().map(|f| {
                false_negative_exceptions.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            println!(
                "Expected exception to match for {} at {}, type {}, got rule match {:?}",
                req.url, req.sourceUrl, req.r#type, checked.filter
            );
        } else if req.blocked == 0 && checked.matched != false {
            mismatch_expected_pass += 1;
            checked.filter.as_ref().map(|f| {
                false_positive_rules.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            println!(
                "Expected pass, matched {} at {}, type {} ON {:?}",
                req.url, req.sourceUrl, req.r#type, checked.filter
            );
        }
    }

    let mismatches = mismatch_expected_match + mismatch_expected_exception + mismatch_expected_pass;
    let ratio = mismatches as f32 / requests_len as f32;
    assert!(ratio < 0.01);
    assert!(
        false_positive_rules.len() < 3,
        "False positive rules higher than expected: {:?}",
        false_positive_rules
    );
    assert!(
        false_negative_rules.len() < 3,
        "False negative rules higher than expected: {:?}",
        false_negative_rules
    );
    assert!(
        false_negative_exceptions.len() < 3,
        "False negative exceptions higher than expected: {:?}",
        false_negative_exceptions
    );
}

#[test]
#[cfg(not(debug_assertions))] // This test takes > 5 minutes to run in debug mode
fn check_matching_hostnames() {
    // Makes sure that requests are handled with the same result whether parsed form full url or from pre-parsed hostname
    let requests = load_requests();

    assert!(requests.len() > 0, "List of parsed request info is empty");

    let engine = get_blocker_engine();

    for req in requests {
        let url_host = adblock::url_parser::parse_url(&req.url).unwrap();
        let source_host = adblock::url_parser::parse_url(&req.sourceUrl).unwrap();
        let domain = url_host.domain();
        let source_domain = source_host.domain();
        let third_party = source_domain != domain;

        let request = Request::new(&req.url, &req.sourceUrl, &req.r#type).unwrap();
        let preparsed_request = Request::preparsed(
            &req.url,
            url_host.hostname(),
            source_host.hostname(),
            &req.r#type,
            third_party,
        );

        let checked = engine.check_network_request(&request);
        let checked_hostnames = engine.check_network_request(&preparsed_request);

        assert_eq!(checked.matched, checked_hostnames.matched);
        assert_eq!(checked.filter, checked_hostnames.filter);
        assert_eq!(checked.exception, checked_hostnames.exception);
        assert_eq!(checked.redirect, checked_hostnames.redirect);
    }
}
