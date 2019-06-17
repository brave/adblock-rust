extern crate criterion;

use criterion::*;

use serde::{Deserialize, Serialize};
use serde_json;

use adblock;
use adblock::url_parser::UrlParser;
use adblock::request::Request;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone)]
struct TestRequest {
    frameUrl: String,
    url: String,
    cpt: String,
}

fn load_requests() -> Vec<TestRequest> {
    adblock::utils::read_rules("data/requests.json")
        .into_iter()
        .map(|r| serde_json::from_str(&r))
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
}

fn request_parsing_throughput(c: &mut Criterion) {
    let requests = load_requests();
    let requests_len = requests.len();
    c.bench(
        "throughput-request",
        Benchmark::new("create", move |b| {
            b.iter(|| {
                let mut successful = 0;
                requests.iter().for_each(|r| {
                    let req: Result<Request, _> =
                        Request::from_urls(&r.url, &r.frameUrl, &r.cpt);
                    if req.is_ok() {
                        successful += 1;
                    }
                })
            })
        })
        .throughput(Throughput::Elements(requests_len as u32))
        .sample_size(10),
    );
}

fn request_extract_hostname(c: &mut Criterion) {
    let requests = load_requests();
    let requests_len = requests.len();
    c.bench(
        "throughput-request",
        Benchmark::new("hostname+domain extract", move |b| {
            b.iter(|| {
                let mut successful = 0;
                requests.iter().for_each(|r| {
                    if Request::parse_url(&r.url).is_some() {
                        successful += 1;
                    }
                    if Request::parse_url(&r.frameUrl).is_some() {
                        successful += 1;
                    }
                });
            })
        })
        .throughput(Throughput::Elements(requests_len as u32))
        .sample_size(10),
    );
}

fn request_new_throughput(c: &mut Criterion) {
    let requests = load_requests();
    let requests_len = requests.len();
    let requests_parsed: Vec<_> = requests.iter().map(|r| {
        let url_norm = r.url.to_ascii_lowercase();
        let source_url_norm = r.frameUrl.to_ascii_lowercase();

        let maybe_parsed_url = Request::parse_url(&url_norm);
        if maybe_parsed_url.is_none() {
            return Err("bad url");
        }
        let parsed_url = maybe_parsed_url.unwrap();

        let maybe_parsed_source = Request::parse_url(&source_url_norm);

        if maybe_parsed_source.is_none() {
            Ok((
                r.cpt.clone(),
                parsed_url.url.clone(),
                String::from(parsed_url.schema()),
                String::from(parsed_url.hostname()),
                String::from(parsed_url.domain()),
                String::from(""),
                String::from(""),
            ))
        } else {
            let parsed_source = maybe_parsed_source.unwrap();
            Ok((
                r.cpt.clone(),
                parsed_url.url.clone(),
                String::from(parsed_url.schema()),
                String::from(parsed_url.hostname()),
                String::from(parsed_url.domain()),
                String::from(parsed_source.hostname()),
                parsed_source.domain().to_owned(),
            ))
        }
    })
    .filter_map(Result::ok)
    .collect();

    c.bench(
        "throughput-request",
        Benchmark::new("new", move |b| {
            b.iter(|| {
                let mut successful = 0;
                requests_parsed.iter().for_each(|r| {
                    Request::new(&r.0, &r.1, &r.2, &r.3, &r.4, &r.5, &r.6);
                    successful += 1;
                });
                
            })
        })
        .throughput(Throughput::Elements(requests_len as u32))
        .sample_size(10),
    );
}

criterion_group!(
    benches,
    request_new_throughput,
    request_extract_hostname,
    request_parsing_throughput
);
criterion_main!(benches);
