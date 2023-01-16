use adblock::engine::Engine;

use serde::Deserialize;

use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct RequestRuleMatch {
    url: String,
    sourceUrl: String,
    r#type: String,
    blocked: u8,
    filter: Option<String>,
}

fn load_requests() -> Vec<RequestRuleMatch> {
    let f = File::open("data/ublock-matches.tsv").expect("file not found");
    let reader = BufReader::new(f);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

    let mut reqs: Vec<RequestRuleMatch> = Vec::new();
    for result in rdr.deserialize() {
        if result.is_ok() {
            let record: RequestRuleMatch = result.expect("WAT");
            reqs.push(record);
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

fn main() {
    println!("Deserializing engine");
    let mut engine;
    {
        let mut file = File::open("data/rs-ABPFilterParserData.dat")
            .expect("Opening serialization file failed");
        let mut serialized = Vec::<u8>::new();
        file.read_to_end(&mut serialized)
            .expect("Reading from serialization file failed");
        engine = Engine::default();
        engine
            .deserialize(&serialized)
            .expect("Deserialization failed");
        // engine = get_blocker_engine();
    }
    engine.use_tags(&["twitter-embeds"]);

    println!("Sleeping");
    std::thread::sleep(std::time::Duration::from_secs(5));

    println!("Loading requests");
    let requests = load_requests();
    let requests_len = requests.len() as u32;
    assert!(requests_len > 0, "List of parsed request info is empty");

    println!("Matching");
    let mut mismatch_expected_match = 0;
    let mut mismatch_expected_exception = 0;
    let mut mismatch_expected_pass = 0;
    let mut false_negative_rules: HashMap<String, (String, String, String)> = HashMap::new();
    let mut false_positive_rules: HashMap<String, (String, String, String)> = HashMap::new();
    let mut false_negative_exceptions: HashMap<String, (String, String, String)> = HashMap::new();
    let mut reqs_processed = 0;
    for req in requests {
        if reqs_processed % 10000 == 0 {
            println!("{} requests processed", reqs_processed);
        }
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        if req.blocked == 1 && checked.matched != true {
            mismatch_expected_match += 1;
            req.filter.as_ref().map(|f| {
                false_negative_rules.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            // println!("Expected match, uBo matched {} at {}, type {} ON {:?}", req.url, req.sourceUrl, req.r#type, req.filter);
        } else if req.blocked == 2 && checked.exception.is_none() {
            mismatch_expected_exception += 1;
            checked.filter.as_ref().map(|f| {
                false_negative_exceptions.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            // println!("Expected exception to match for {} at {}, type {}, got rule match {:?}", req.url, req.sourceUrl, req.r#type, checked.filter);
        } else if req.blocked == 0 && checked.matched != false {
            mismatch_expected_pass += 1;
            checked.filter.as_ref().map(|f| {
                false_positive_rules.insert(
                    f.clone(),
                    (req.url.clone(), req.sourceUrl.clone(), req.r#type.clone()),
                )
            });
            // println!("Expected pass, matched {} at {}, type {} ON {:?}", req.url, req.sourceUrl, req.r#type, checked.filter);
        }
        reqs_processed += 1;
    }

    let mismatches = mismatch_expected_match + mismatch_expected_exception + mismatch_expected_pass;
    let ratio = mismatches as f32 / requests_len as f32;
    assert!(ratio < 0.04, "Mismatch ratio was {}", ratio);
    assert!(
        false_positive_rules.len() < 3,
        "False positive rules higher than expected: {:?}",
        false_positive_rules.len()
    );
    assert!(
        false_negative_rules.len() < 70,
        "False negative rules higher than expected: {:?}",
        false_negative_rules.len()
    );
    assert!(
        false_negative_exceptions.len() < 3,
        "False negative exceptions higher than expected: {:?}",
        false_negative_exceptions.len()
    );
}
