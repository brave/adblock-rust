extern crate criterion;

use criterion::*;

use serde::{Deserialize, Serialize};
use serde_json;

use adblock;
use adblock::url_parser::UrlParser;

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct TestRequest {
    frameUrl: String,
    url: String,
    cpt: String,
}

fn load_requests() -> Vec<TestRequest> {
    let requests_str = adblock::utils::read_rules("data/requests.json");
    let reqs: Vec<TestRequest> = requests_str
        .into_iter()
        .map(|r| serde_json::from_str(&r))
        .filter_map(Result::ok)
        .collect();
    reqs
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
                    let req: Result<adblock::request::Request, _> =
                        adblock::request::Request::from_urls(&r.url, &r.frameUrl, &r.cpt);
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
                    if adblock::request::Request::get_url_host(&r.url).is_some() {
                        successful += 1;
                    }
                    if adblock::request::Request::get_url_host(&r.frameUrl).is_some() {
                        successful += 1;
                    }
                });
            })
        })
        .throughput(Throughput::Elements(requests_len as u32))
        .sample_size(10),
    );
}

criterion_group!(
    benches,
    request_extract_hostname,
    request_parsing_throughput,
    // host_throughput,
    // domain_throughput
);
criterion_main!(benches);
