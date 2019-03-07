#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::ParameterizedBenchmark;
use criterion::Throughput;

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
            |b, url| b.iter(|| adblock::get_url_host(url)),
            URLS,
        ).throughput(|_url| Throughput::Elements(1)),
    );
}

fn url_domain_throughput(c: &mut Criterion) {
  c.bench(
        "throughput-url-domain",
        ParameterizedBenchmark::new(
            "get domain",
            |b, url| b.iter(|| adblock::get_url_domain(url)),
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
              let host = adblock::get_url_host(&url).unwrap();
              b.iter(|| adblock::get_host_domain(&host))
            },
            URLS,
        ).throughput(|_url| Throughput::Elements(1)),
    );
}

criterion_group!(benches, host_throughput, url_domain_throughput, domain_throughput);
criterion_main!(benches);
