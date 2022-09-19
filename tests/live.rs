use adblock::engine::Engine;

use serde::Deserialize;
use tokio::runtime::Runtime;

use std::fs::File;
use std::io::BufReader;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct RequestRuleMatch {
    url: String,
    sourceUrl: String,
    r#type: String,
    blocked: bool
}

fn load_requests() -> Vec<RequestRuleMatch> {
    let f = File::open("data/regressions.tsv").expect("file not found");
    let reader = BufReader::new(f);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

    let mut reqs: Vec<RequestRuleMatch> = Vec::new();
    for result in rdr.deserialize() {
        if result.is_ok() {
            let record: RequestRuleMatch = result.unwrap();
            reqs.push(RequestRuleMatch {
                url: record.url.trim_matches('"').to_owned(),
                sourceUrl: record.sourceUrl.trim_matches('"').to_owned(),
                r#type: record.r#type.trim_matches('"').to_owned(),
                blocked: record.blocked
            });
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

/// Describes an online source of adblock rules.
/// https://github.com/brave/adblock-resources#filter-list-description-format
#[derive(serde::Deserialize, Debug)]
pub struct RemoteFilterSource {
    pub url: String,
    pub title: String,
    pub format: adblock::lists::FilterFormat,
    #[serde(default)] // should default to false if the field is missing
    pub include_redirect_urls: bool,
    pub support_url: String,
}

/// Fetch all filters once and store them in a lazy-loaded static variable to avoid unnecessary
/// network traffic.
static ALL_FILTERS: once_cell::sync::Lazy<std::sync::Mutex<adblock::lists::FilterSet>> = once_cell::sync::Lazy::new(|| {
    async fn get_all_filters() -> adblock::lists::FilterSet {
        use futures::FutureExt;

        const DEFAULT_LISTS_URL: &'static str = "https://raw.githubusercontent.com/brave/adblock-resources/master/filter_lists/default.json";

        println!("Downloading list of filter lists from '{}'", DEFAULT_LISTS_URL);
        let default_lists: Vec<RemoteFilterSource> = async {
            let body = reqwest::get(DEFAULT_LISTS_URL).await.unwrap().text().await.unwrap();
            serde_json::from_str(&body).unwrap()
        }.await;
        println!("List of filter lists has {} filter lists total", default_lists.len());
        // ↑ Question about that:
        // How many filter lists could a list of filter lists list if a list of filter lists could list filter lists?
        // (asking for a friend, who may or may not be a woodchuck)

        assert!(default_lists.len() > 10); // sanity check

        let filters_fut: Vec<_> = default_lists
            .iter()
            .map(|list| {
                println!("Starting download of filter, '{}'", list.url);
                // 'easylist.to' is deployed via GitHub Pages. However, sometimes 'easylist.to' can
                // take minutes to respond despite 'easylist.github.io' having no delay.
                //
                // In one test, the first request below ↓ took <1 second, and the second request took ~7 minutes
                // time curl --fail -H 'Host: easylist.to' https://easylist.github.io/easylist/easylist.txt -o /dev/null
                // time curl --fail https://easylist.to/easylist/easylist.txt -o /dev/null
                //
                // Toggle the `cfg` below if encountering this issue during local development.
                // (e.g., add '--cfg override_easylist_host' to RUSTFLAGS)
                #[cfg(any(override_easylist_host))]
                let downloader = {
                    let client = reqwest::Client::builder()
                        .redirect(reqwest::redirect::Policy::none())
                        .build()
                        .unwrap();
                    if list.url.starts_with("https://easylist.to") {
                        // The use of 'http' rather than 'https' below is intentional. reqwest only
                        // respects the host header if the target url is http, not https. (Unclear
                        // whether that's a bug or intentional behavior.) One way to confirm that is
                        // to send requests to http://httpbin.org/headers instead of github.io.
                        client
                            .get(&list.url.replace("https://easylist.to", "http://easylist.github.io"))
                            .header(reqwest::header::HOST, "easylist.to")
                            .send()
                    } else {
                        // leave all other filter list requests unmodified
                        client.get(&list.url).send()
                    }
                };

                #[cfg(not(override_easylist_host))]
                let downloader = reqwest::get(&list.url);
                downloader
                    .then(move |resp| {
                        let response = resp.expect("Could not request rules");
                        if response.status() != 200 {
                            panic!("Failed download of filter, '{}'. Received status code {} when only 200 was expected", list.url.clone(), response.status());
                        }
                        response.text()
                    }).map(move |text| {
                        let text = text.expect("Could not get rules as text");
                        println!("Finished download of filter, '{}' ({} bytes)", list.url, text.len());
                        // Troubleshooting tip: uncomment the next line to save the downloaded filter lists
                        // std::fs::write(format!("target/{}.txt", list.title), &text).unwrap();
                        ( list.format, text )
                    })
            })
            .collect();

        // Troubleshooting tip: replace default() below with new(true) to expose raw filters
        let mut filter_set = adblock::lists::FilterSet::default();

        futures::future::join_all(filters_fut)
            .await
            .iter()
            .for_each(|(format, list)| {
                filter_set.add_filters(&list.lines().map(|s| s.to_owned()).collect::<Vec<_>>(), adblock::lists::ParseOptions { format: *format, ..Default::default() });
            });

        filter_set
    }

    let async_runtime = Runtime::new().expect("Could not start Tokio runtime");
    std::sync::Mutex::new(async_runtime.block_on(get_all_filters()))
});

/// Example usage of this test:
///
/// cargo watch --clear -x "test --all-features --test live -- --show-output --nocapture --include-ignored 'troubleshoot'"
#[test]
#[ignore = "opt-in: used for troubleshooting issues with live tests"]
fn troubleshoot() {
    println!("Troubleshooting initiated. Safe journeys. ⛵");
    let _grabbed = ALL_FILTERS.lock().unwrap();
    println!("Troubleshooting complete. Welcome back! 🥳");
}

fn get_blocker_engine() -> Engine {
    let mut engine = Engine::from_filter_set(ALL_FILTERS.lock().unwrap().clone(), true);

    engine.use_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn get_blocker_engine_deserialized() -> Engine {
    use futures::FutureExt;
    let async_runtime = Runtime::new().expect("Could not start Tokio runtime");

    let brave_service_key = std::env::var("BRAVE_SERVICE_KEY")
        .expect("Must set the $BRAVE_SERVICE_KEY environment variable to execute live tests.");

    let dat_url = "https://adblock-data.s3.brave.com/4/rs-ABPFilterParserData.dat";
    let download_client = reqwest::Client::new();
    let resp_bytes_fut = download_client.get(dat_url)
        .header("BraveServiceKey", brave_service_key)
        .send()
        .map(|e| e.expect("Could not request rules"))
        .then(|resp| {
            assert_eq!(resp.status(), 200, "Downloading live DAT failed. Is the service key correct?");
            resp.bytes()
        });
    let dat = async_runtime
        .block_on(resp_bytes_fut)
        .expect("Could not get response as bytes");

    let mut engine = Engine::default();
    engine.deserialize(&dat).expect("Deserialization failed");
    engine.use_tags(&["fb-embeds", "twitter-embeds"]);
    engine
}

fn get_blocker_engine_deserialized_ios() -> Engine {
    use futures::FutureExt;
    let async_runtime = Runtime::new().expect("Could not start Tokio runtime");

    let list_url = "https://adblock-data.s3.brave.com/ios/latest.txt";
    let resp_text_fut = reqwest::get(list_url)
        .map(|resp| resp.expect("Could not request rules"))
        .then(|resp| resp.text());
    let filters: Vec<String> = async_runtime
        .block_on(resp_text_fut)
        .expect("Could not get rules as text")
        .lines()
        .map(|s| s.to_owned())
        .collect();

    let engine = Engine::from_rules_parametrised(&filters, Default::default(), true, false);
    engine
}

#[test]
fn check_live_specific_urls() {
    let mut engine = get_blocker_engine();
    {
        let checked = engine.check_network_urls(
            "https://static.scroll.com/js/scroll.js",
            "https://www.theverge.com/",
            "script");
        assert_eq!(checked.matched, false,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
    }
    {
        engine.disable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_urls(
            "https://platform.twitter.com/widgets.js",
            "https://fmarier.github.io/brave-testing/social-widgets.html",
            "script");
        assert_eq!(checked.matched, true,
            "Expected no match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        engine.enable_tags(&["twitter-embeds"]);
    }
    {
        engine.disable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_urls(
            "https://imagesrv.adition.com/banners/1337/files/00/0e/6f/09/000000945929.jpg?PQgSgs13hf1fw.jpg",
            "https://spiegel.de",
            "image");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        engine.enable_tags(&["twitter-embeds"]);
    }
}

#[test]
#[ignore = "opt-in: requires BRAVE_SERVICE_KEY environment variable"]
fn check_live_brave_deserialized_specific_urls() { // Note: CI relies on part of this function's name
    let mut engine = get_blocker_engine_deserialized();
    {
        engine.disable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_urls(
            "https://platform.twitter.com/widgets.js",
            "https://fmarier.github.io/brave-testing/social-widgets.html",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
    }
    {
        engine.enable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_urls(
            "https://platform.twitter.com/widgets.js",
            "https://fmarier.github.io/brave-testing/social-widgets.html",
            "script");
        assert_eq!(checked.matched, false,
            "Expected no match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
    }
}

#[test]
fn check_live_from_filterlists() {
    let engine = get_blocker_engine();
    let requests = load_requests();

    for req in requests {
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} at {}, got filter {:?}, exception {:?}",
            req.blocked, req.url, req.sourceUrl, checked.filter, checked.exception);
    }
}

#[test]
#[ignore = "opt-in: requires BRAVE_SERVICE_KEY environment variable"]
fn check_live_brave_deserialized_file() { // Note: CI relies on part of this function's name
    let engine = get_blocker_engine_deserialized();
    let requests = load_requests();

    for req in requests {
        println!("Checking {:?}", req);
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} {} {}",
            req.blocked, req.url, req.sourceUrl, req.r#type);
    }
}

/// This test always fails, and likely has never succeeded, because every instance of this function
/// across all commits, has always appeared right after '#[ignore]'.
///
/// git --no-pager grep -B1 "check_live_deserialized_ios" $(git rev-list --all)
#[test]
#[ignore = "ever since I was created 😿"]
fn check_live_deserialized_ios() {
    let engine = get_blocker_engine_deserialized_ios();
    let requests = load_requests();

    for req in requests {
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} {} {}",
            req.blocked, req.url, req.sourceUrl, req.r#type);
    }
}

#[cfg(feature = "resource-assembler")]
#[test]
fn check_live_redirects() {
    use adblock::resources::resource_assembler::assemble_web_accessible_resources;

    let mut engine = get_blocker_engine();
    let redirect_engine_path = std::path::Path::new("data/test/fake-uBO-files/redirect-engine.js");
    let war_dir = std::path::Path::new("data/test/fake-uBO-files/web_accessible_resources");
    let resources = assemble_web_accessible_resources(war_dir, redirect_engine_path);

    engine.use_resources(&resources);
    {
        let checked = engine.check_network_urls(
            "https://c.amazon-adsystem.com/aax2/amzn_ads.js",
            "https://aussieexotics.com/",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        assert!(checked.redirect.is_some(), "{:#?}", checked);
        // Check for the specific expected return script value in base64
        match checked.redirect.unwrap() {
            adblock::blocker::Redirection::Resource(data) => assert_eq!(data, "data:application/javascript;base64,LyoqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioKCiAgICB1QmxvY2sgT3JpZ2luIC0gYSBicm93c2VyIGV4dGVuc2lvbiB0byBibG9jayByZXF1ZXN0cy4KICAgIENvcHlyaWdodCAoQykgMjAxOS1wcmVzZW50IFJheW1vbmQgSGlsbAoKICAgIFRoaXMgcHJvZ3JhbSBpcyBmcmVlIHNvZnR3YXJlOiB5b3UgY2FuIHJlZGlzdHJpYnV0ZSBpdCBhbmQvb3IgbW9kaWZ5CiAgICBpdCB1bmRlciB0aGUgdGVybXMgb2YgdGhlIEdOVSBHZW5lcmFsIFB1YmxpYyBMaWNlbnNlIGFzIHB1Ymxpc2hlZCBieQogICAgdGhlIEZyZWUgU29mdHdhcmUgRm91bmRhdGlvbiwgZWl0aGVyIHZlcnNpb24gMyBvZiB0aGUgTGljZW5zZSwgb3IKICAgIChhdCB5b3VyIG9wdGlvbikgYW55IGxhdGVyIHZlcnNpb24uCgogICAgVGhpcyBwcm9ncmFtIGlzIGRpc3RyaWJ1dGVkIGluIHRoZSBob3BlIHRoYXQgaXQgd2lsbCBiZSB1c2VmdWwsCiAgICBidXQgV0lUSE9VVCBBTlkgV0FSUkFOVFk7IHdpdGhvdXQgZXZlbiB0aGUgaW1wbGllZCB3YXJyYW50eSBvZgogICAgTUVSQ0hBTlRBQklMSVRZIG9yIEZJVE5FU1MgRk9SIEEgUEFSVElDVUxBUiBQVVJQT1NFLiAgU2VlIHRoZQogICAgR05VIEdlbmVyYWwgUHVibGljIExpY2Vuc2UgZm9yIG1vcmUgZGV0YWlscy4KCiAgICBZb3Ugc2hvdWxkIGhhdmUgcmVjZWl2ZWQgYSBjb3B5IG9mIHRoZSBHTlUgR2VuZXJhbCBQdWJsaWMgTGljZW5zZQogICAgYWxvbmcgd2l0aCB0aGlzIHByb2dyYW0uICBJZiBub3QsIHNlZSB7aHR0cDovL3d3dy5nbnUub3JnL2xpY2Vuc2VzL30uCgogICAgSG9tZTogaHR0cHM6Ly9naXRodWIuY29tL2dvcmhpbGwvdUJsb2NrCiovCgooZnVuY3Rpb24oKSB7CiAgICAndXNlIHN0cmljdCc7CiAgICBpZiAoIGFtem5hZHMgKSB7CiAgICAgICAgcmV0dXJuOwogICAgfQogICAgdmFyIHcgPSB3aW5kb3c7CiAgICB2YXIgbm9vcGZuID0gZnVuY3Rpb24oKSB7CiAgICAgICAgOwogICAgfS5iaW5kKCk7CiAgICB2YXIgYW16bmFkcyA9IHsKICAgICAgICBhcHBlbmRTY3JpcHRUYWc6IG5vb3BmbiwKICAgICAgICBhcHBlbmRUYXJnZXRpbmdUb0FkU2VydmVyVXJsOiBub29wZm4sCiAgICAgICAgYXBwZW5kVGFyZ2V0aW5nVG9RdWVyeVN0cmluZzogbm9vcGZuLAogICAgICAgIGNsZWFyVGFyZ2V0aW5nRnJvbUdQVEFzeW5jOiBub29wZm4sCiAgICAgICAgZG9BbGxUYXNrczogbm9vcGZuLAogICAgICAgIGRvR2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICBkb1Rhc2s6IG5vb3BmbiwKICAgICAgICBkZXRlY3RJZnJhbWVBbmRHZXRVUkw6IG5vb3BmbiwKICAgICAgICBnZXRBZHM6IG5vb3BmbiwKICAgICAgICBnZXRBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldEFkRm9yU2xvdDogbm9vcGZuLAogICAgICAgIGdldEFkc0NhbGxiYWNrOiBub29wZm4sCiAgICAgICAgZ2V0RGlzcGxheUFkczogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGdldEtleXM6IG5vb3BmbiwKICAgICAgICBnZXRSZWZlcnJlclVSTDogbm9vcGZuLAogICAgICAgIGdldFNjcmlwdFNvdXJjZTogbm9vcGZuLAogICAgICAgIGdldFRhcmdldGluZzogbm9vcGZuLAogICAgICAgIGdldFRva2Vuczogbm9vcGZuLAogICAgICAgIGdldFZhbGlkTWlsbGlzZWNvbmRzOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHM6IG5vb3BmbiwKICAgICAgICBnZXRWaWRlb0Fkc0FzeW5jOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGhhbmRsZUNhbGxCYWNrOiBub29wZm4sCiAgICAgICAgaGFzQWRzOiBub29wZm4sCiAgICAgICAgcmVuZGVyQWQ6IG5vb3BmbiwKICAgICAgICBzYXZlQWRzOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nRm9yR1BUQXN5bmM6IG5vb3BmbiwKICAgICAgICBzZXRUYXJnZXRpbmdGb3JHUFRTeW5jOiBub29wZm4sCiAgICAgICAgdHJ5R2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICB1cGRhdGVBZHM6IG5vb3BmbgogICAgfTsKICAgIHcuYW16bmFkcyA9IGFtem5hZHM7CiAgICB3LmFtem5fYWRzID0gdy5hbXpuX2FkcyB8fCBub29wZm47CiAgICB3LmFheF93cml0ZSA9IHcuYWF4X3dyaXRlIHx8IG5vb3BmbjsKICAgIHcuYWF4X3JlbmRlcl9hZCA9IHcuYWF4X3JlbmRlcl9hZCB8fCBub29wZm47Cn0pKCk7Cg=="),
            adblock::blocker::Redirection::Url(_) => unreachable!(),
        }
    }
    {
        let checked = engine.check_network_urls(
            "https://www.googletagservices.com/tag/js/gpt.js",
            "https://tvguide.com/",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        assert!(checked.redirect.is_some(), "{:#?}", checked);
        match checked.redirect.unwrap() {
            adblock::blocker::Redirection::Resource(data) => assert_eq!(data, "data:application/javascript;base64,KGZ1bmN0aW9uKCkgewogICAgJ3VzZSBzdHJpY3QnOwp9KSgpOwo="),
            adblock::blocker::Redirection::Url(_) => unreachable!(),
        }
    }
}

#[test]
/// Ensure that two different engines loaded from the same textual filter set serialize to
/// identical buffers.
fn stable_serialization() {
    let engine1 = Engine::from_filter_set(ALL_FILTERS.lock().unwrap().clone(), true);
    let ser1 = engine1.serialize_raw().unwrap();

    let engine2 = Engine::from_filter_set(ALL_FILTERS.lock().unwrap().clone(), true);
    let ser2 = engine2.serialize_raw().unwrap();

    assert_eq!(ser1, ser2);
}

#[test]
/// Ensure that one engine's serialization result can be exactly reproduced by another engine after
/// deserializing from it.
fn stable_serialization_through_load() {
    let engine1 = Engine::from_filter_set(ALL_FILTERS.lock().unwrap().clone(), true);
    let ser1 = engine1.serialize_raw().unwrap();

    let mut engine2 = Engine::new(true);
    engine2.deserialize(&ser1).unwrap();
    let ser2 = engine2.serialize_raw().unwrap();

    assert_eq!(ser1, ser2);
}
