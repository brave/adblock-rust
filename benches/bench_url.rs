extern crate criterion;

use criterion::*;

use adblock;

static URLS: &'static [&'static str] = &[
    // No public suffix
    "http://example.foo.edu.au", // null
    "http://example.foo.edu.sh", // null
    "http://example.disrec.thingdust.io", // null
    "http://foo.bar.baz.ortsinfo.at", // null

    // ICANN
    "http://example.foo.nom.br", // *.nom.br
    "http://example.wa.edu.au", // wa.edu.au
    "http://example.com", // com
    "http://example.co.uk", // co.uk

    // Private
    "http://foo.bar.baz.stolos.io", // *.stolos.io
    "http://foo.art.pl", // art.pl
    "http://foo.privatizehealthinsurance.net", // privatizehealthinsurance.net
    "http://example.cust.disrec.thingdust.io", // cust.disrec.thingdust.io

    // Exception
    "http://foo.city.kitakyushu.jp", // !city.kitakyushu.jp
    "http://example.www.ck", // !www.ck
    "http://foo.bar.baz.city.yokohama.jp", // !city.yokohama.jp
    "http://example.city.kobe.jp", // !city.kobe.jp

    "http://www.google.com",
    "http://forums.news.cnn.com"
];

fn host_throughput(c: &mut Criterion) {
    c.bench(
        "throughput-host",
        ParameterizedBenchmark::new(
            "get hostname",
            |b, url| b.iter(|| adblock::request::get_url_host(url)),
            URLS,
        ).throughput(|_url| Throughput::Elements(1)),
    );
}

fn url_domain_throughput(c: &mut Criterion) {
    c.bench(
        "throughput-url-domain",
        ParameterizedBenchmark::new(
            "get domain",
            |b, url| b.iter(|| adblock::request::get_url_domain(url)),
            URLS,
        ).throughput(|_url| Throughput::Elements(1)),
    );
}


fn domain_throughput(c: &mut Criterion) {
    c.bench(
        "throughput-domain",
        ParameterizedBenchmark::new(
            "get domain",
            |b, url| {
                let host = adblock::request::get_url_host(&url).unwrap();
                b.iter(|| adblock::request::get_host_domain(&host))
            },
            URLS,
        ).throughput(|_url| Throughput::Elements(1)),
    );
}

use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct TestRequest {
    frameUrl: String,
    url: String,
    cpt: String
}

fn load_requests() -> Vec<TestRequest> {
    let requests_str = adblock::utils::read_rules("data/requests.json");
    let reqs: Vec<TestRequest> = requests_str.into_iter().map(|r| serde_json::from_str(&r)).filter_map(Result::ok).collect();
    reqs
}

fn parse_requests(requests: &Vec<TestRequest>) -> u32 {
    requests
    .iter()
    .map(|r| {
        let req: Result<adblock::request::Request, _> = adblock::request::Request::from_urls(&r.url, &r.frameUrl, &r.cpt);
        req
    })
    .filter_map(Result::ok)
    .map(|r| r.source_hostname_hash)
    .fold(0u32, |acc, r| acc ^ r)
}


fn request_parsing_throughput(c: &mut Criterion) {
    let requests = load_requests();
    let requests_len = requests.len();
    c.bench(
        "throughput-request-create",
        Benchmark::new(
            "parse requests",
            move |b| b.iter(|| parse_requests(&requests)),
        ).throughput(Throughput::Elements(requests_len as u32)),
    );
}

criterion_group!(benches, request_parsing_throughput, host_throughput, url_domain_throughput, domain_throughput);
criterion_main!(benches);
