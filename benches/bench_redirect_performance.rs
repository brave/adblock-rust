use criterion::*;
use once_cell::sync::Lazy;
use psl::Psl;
use tokio::runtime::Runtime;

use std::path::Path;

use adblock::filters::network::{NetworkFilter, NetworkFilterMask};
use adblock::request::Request;
use adblock::blocker::{Blocker, BlockerOptions};
use adblock::resources::resource_assembler::{assemble_web_accessible_resources, assemble_scriptlet_resources};

static PSL_LIST: Lazy<psl::List> = Lazy::new(|| psl::List::new());

async fn get_all_filters() -> Vec<String> {
    use futures::FutureExt;
    let default_lists = adblock::filter_lists::default::default_lists();

    let filters_fut: Vec<_> = default_lists
        .iter()
        .map(|list| {
            reqwest::get(&list.url)
                .then(|resp| resp
                    .expect("Could not request rules")
                    .text()
                ).map(|text| text
                    .expect("Could not get rules as text")
                    .lines()
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>()
                )
        })
        .collect();

    futures::future::join_all(filters_fut)
        .await
        .iter()
        .flatten()
        .cloned()
        .collect()
}

/// Gets all rules with redirects, and modifies them to apply to resources at `a{0-n}.com/bad.js`
fn get_redirect_rules() -> Vec<NetworkFilter> {
    let mut async_runtime = Runtime::new().expect("Could not start Tokio runtime");

    let filters = async_runtime.block_on(get_all_filters());
    let (network_filters, _) = adblock::lists::parse_filters(&filters, true, adblock::lists::FilterFormat::Standard);

    network_filters.into_iter()
        .filter(|rule| {
            if let Some(ref redirect) = rule.redirect {
                if redirect != "none" {
                    return true;
                }
            }
            false
        })
        .enumerate()
        .map(|(index, mut rule)| {
            rule.mask.insert(NetworkFilterMask::IS_LEFT_ANCHOR);
            rule.mask.insert(NetworkFilterMask::IS_RIGHT_ANCHOR);
            rule.hostname = Some(format!("a{}.com/bad.js", index));

            rule.filter = adblock::filters::network::FilterPart::Empty;
            rule.mask.remove(NetworkFilterMask::IS_HOSTNAME_ANCHOR);
            rule.mask.remove(NetworkFilterMask::IS_HOSTNAME_REGEX);
            rule.mask.remove(NetworkFilterMask::IS_REGEX);
            rule.mask.remove(NetworkFilterMask::IS_COMPLETE_REGEX);
            rule.mask.remove(NetworkFilterMask::FUZZY_MATCH);

            rule
        })
        .collect()
}

/// Loads the supplied rules, and the test set of resources, into a Blocker
fn get_preloaded_blocker(rules: Vec<NetworkFilter>) -> Blocker {
    let blocker_options = BlockerOptions {
        enable_optimizations: true,
    };

    let mut blocker = Blocker::new(rules, &blocker_options);

    let mut resources = assemble_web_accessible_resources(
        Path::new("data/test/fake-uBO-files/web_accessible_resources"),
        Path::new("data/test/fake-uBO-files/redirect-engine.js")
    );
    resources.append(&mut assemble_scriptlet_resources(
        Path::new("data/test/fake-uBO-files/scriptlets.js"),
    ));

    blocker.use_resources(&resources);

    blocker
}

/// Maps network filter rules into `Request`s that would trigger those rules
pub fn build_custom_requests(rules: Vec<NetworkFilter>) -> Vec<Request> {
    rules.iter().map(|rule| {
        let raw_type = if rule.mask.contains(NetworkFilterMask::FROM_IMAGE) {
            "image"
        } else if rule.mask.contains(NetworkFilterMask::FROM_MEDIA) {
            "media"
        } else if rule.mask.contains(NetworkFilterMask::FROM_OBJECT) {
            "object"
        } else if rule.mask.contains(NetworkFilterMask::FROM_OTHER) {
            "other"
        } else if rule.mask.contains(NetworkFilterMask::FROM_PING) {
            "ping"
        } else if rule.mask.contains(NetworkFilterMask::FROM_SCRIPT) {
            "script"
        } else if rule.mask.contains(NetworkFilterMask::FROM_STYLESHEET) {
            "stylesheet"
        } else if rule.mask.contains(NetworkFilterMask::FROM_SUBDOCUMENT) {
            "subdocument"
        } else if rule.mask.contains(NetworkFilterMask::FROM_DOCUMENT) {
            "main_frame"
        } else if rule.mask.contains(NetworkFilterMask::FROM_XMLHTTPREQUEST) {
            "xhr"
        } else if rule.mask.contains(NetworkFilterMask::FROM_WEBSOCKET) {
            "websocket"
        } else if rule.mask.contains(NetworkFilterMask::FROM_FONT) {
            "font"
        } else {
            unreachable!()
        };

        let rule_hostname = rule.hostname.clone().unwrap();
        let url = format!("https://{}", rule_hostname.clone());
        let domain = &rule_hostname[..rule_hostname.find('/').unwrap()];
        let hostname = domain;

        let raw_line = rule.raw_line.clone().unwrap();
        let (source_hostname, source_domain) = if rule.opt_domains.is_some() {
            let domain_start = raw_line.rfind("domain=").unwrap() + "domain=".len();
            let from_start = &raw_line[domain_start..];
            let domain_end = from_start.find('|').or_else(|| from_start.find(",")).or_else(|| Some(from_start.len())).unwrap() + domain_start;
            let source_hostname = &raw_line[domain_start..domain_end];

            let suffix = PSL_LIST.suffix(source_hostname).unwrap();
            let suffix = suffix.to_str();
            let domain_start = source_hostname[..source_hostname.len()-suffix.len()-1].rfind('.');
            let source_domain = if let Some(domain_start) = domain_start {
                &source_hostname[domain_start+1..]
            } else {
                source_hostname
            };
            (source_hostname, source_domain)
        } else {
            (hostname, domain)
        };

        Request::new(
            raw_type,
            &url,
            "https",
            hostname,
            domain,
            source_hostname,
            source_domain,
        )
    }).collect::<Vec<_>>()
}

fn bench_fn(blocker: &Blocker, requests: &[Request]) {
    requests.iter().for_each(|request| {
        let block_result = blocker.check(&request);
        assert!(block_result.redirect.is_some());
    });
}

fn redirect_performance(c: &mut Criterion) {
    let rules = get_redirect_rules();

    let blocker = get_preloaded_blocker(rules.clone());
    let requests = build_custom_requests(rules.clone());
    let requests_len = requests.len() as u32;

    c.bench(
        "redirect_performance",
        Benchmark::new(
            "without_alias_lookup",
            move |b| {
                b.iter(|| bench_fn(&blocker, &requests))
            },
        ).throughput(Throughput::Elements(requests_len))
        .sample_size(10)
    );
}

criterion_group!(
  benches,
  redirect_performance,
);
criterion_main!(benches);
