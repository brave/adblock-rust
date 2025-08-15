use adblock::request::Request;
use adblock::Engine;

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
    blocked: bool,
}

fn load_requests() -> Vec<RequestRuleMatch> {
    let f = File::open("data/regressions.tsv").expect("file not found");
    let reader = BufReader::new(f);
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);

    let mut reqs: Vec<RequestRuleMatch> = Vec::new();
    for result in rdr.deserialize::<RequestRuleMatch>() {
        if let Ok(record) = result {
            reqs.push(RequestRuleMatch {
                url: record.url.trim_matches('"').to_owned(),
                sourceUrl: record.sourceUrl.trim_matches('"').to_owned(),
                r#type: record.r#type.trim_matches('"').to_owned(),
                blocked: record.blocked,
            });
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

/// Describes an entry from Brave's catalog of adblock lists.
/// https://github.com/brave/adblock-resources#filter-list-description-format
#[derive(serde::Deserialize, Debug)]
pub struct RemoteFilterCatalogEntry {
    pub title: String,
    #[serde(default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub platforms: Vec<String>,
    pub sources: Vec<RemoteFilterSource>,
}

/// Describes an online source of adblock rules. Corresponds to a single entry of `sources` as
/// defined [here](https://github.com/brave/adblock-resources#filter-list-description-format).
#[derive(serde::Deserialize, Debug)]
pub struct RemoteFilterSource {
    pub url: String,
    pub title: Option<String>,
    pub format: adblock::lists::FilterFormat,
    pub support_url: String,
}

/// Fetch all filters once and store them in a lazy-loaded static variable to avoid unnecessary
/// network traffic.
static ALL_FILTERS: once_cell::sync::Lazy<std::sync::Mutex<adblock::lists::FilterSet>> =
    once_cell::sync::Lazy::new(|| {
        async fn get_all_filters() -> adblock::lists::FilterSet {
            use futures::FutureExt;

            const DEFAULT_LISTS_URL: &str = "https://raw.githubusercontent.com/brave/adblock-resources/master/filter_lists/list_catalog.json";

            println!(
                "Downloading list of filter lists from '{}'",
                DEFAULT_LISTS_URL
            );
            let default_catalog: Vec<RemoteFilterCatalogEntry> = async {
                let body = reqwest::get(DEFAULT_LISTS_URL)
                    .await
                    .unwrap()
                    .text()
                    .await
                    .unwrap();
                serde_json::from_str(&body).unwrap()
            }
            .await;

            let default_lists: Vec<_> = default_catalog
                .iter()
                .filter(|comp| comp.default_enabled)
                .filter(|comp| {
                    comp.platforms.is_empty()
                        || comp.platforms.iter().any(|platform| {
                            ["LINUX", "WINDOWS", "MAC"].contains(&platform.as_str())
                        })
                })
                .flat_map(|comp| &comp.sources)
                .collect();

            assert!(default_lists.len() > 10); // sanity check

            let filters_fut: Vec<_> = default_lists
            .iter()
            .map(|list| {
                println!("Starting download of filter, '{}'", list.url);
                reqwest::get(&list.url)
                    .then(move |resp| {
                        let response = resp.expect("Could not request rules");
                        if response.status() != 200 {
                            panic!("Failed download of filter, '{}'. Received status code {} when only 200 was expected", list.url.clone(), response.status());
                        }
                        response.text()
                    }).map(move |text| {
                        let text = text.expect("Could not get rules as text");
                        println!("Finished download of filter, '{}' ({} bytes)", list.url, text.len());
                        ( list.format, text )
                    })
            })
            .collect();

            let mut filter_set = adblock::lists::FilterSet::default();

            futures::future::join_all(filters_fut)
                .await
                .iter()
                .for_each(|(format, list)| {
                    filter_set.add_filters(
                        list.lines().map(|s| s.to_owned()).collect::<Vec<_>>(),
                        adblock::lists::ParseOptions {
                            format: *format,
                            ..Default::default()
                        },
                    );
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

#[test]
fn check_live_specific_urls() {
    let mut engine = get_blocker_engine();
    {
        let checked = engine.check_network_request(
            &Request::new(
                "https://static.scroll.com/js/scroll.js",
                "https://www.theverge.com/",
                "script",
            )
            .unwrap(),
        );
        assert!(
            !checked.matched,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception
        );
    }
    {
        engine.disable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_request(
            &Request::new(
                "https://platform.twitter.com/widgets.js",
                "https://fmarier.github.io/brave-testing/social-widgets.html",
                "script",
            )
            .unwrap(),
        );
        assert!(
            checked.matched,
            "Expected no match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception
        );
        engine.enable_tags(&["twitter-embeds"]);
    }
    {
        engine.disable_tags(&["twitter-embeds"]);
        let checked = engine.check_network_request(&Request::new(
            "https://imagesrv.adition.com/banners/1337/files/00/0e/6f/09/000000945929.jpg?PQgSgs13hf1fw.jpg",
            "https://spiegel.de",
            "image",
        ).unwrap());
        assert!(
            checked.matched,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception
        );
        engine.enable_tags(&["twitter-embeds"]);
    }
}

#[test]
fn check_live_from_filterlists() {
    let engine = get_blocker_engine();
    let requests = load_requests();

    for req in requests {
        let checked = engine
            .check_network_request(&Request::new(&req.url, &req.sourceUrl, &req.r#type).unwrap());
        assert_eq!(
            checked.matched, req.blocked,
            "Expected match {} for {} at {}, got filter {:?}, exception {:?}",
            req.blocked, req.url, req.sourceUrl, checked.filter, checked.exception
        );
    }
}

#[cfg(feature = "resource-assembler")]
#[test]
#[ignore = "issues/499"]
fn check_live_redirects() {
    use adblock::resources::resource_assembler::assemble_web_accessible_resources;

    let mut engine = get_blocker_engine();
    let redirect_engine_path =
        std::path::Path::new("data/test/fake-uBO-files/redirect-resources.js");
    let war_dir = std::path::Path::new("data/test/fake-uBO-files/web_accessible_resources");
    let resources = assemble_web_accessible_resources(war_dir, redirect_engine_path);

    engine.use_resources(resources);
    {
        let checked = engine.check_network_request(
            &Request::new(
                "https://c.amazon-adsystem.com/aax2/amzn_ads.js",
                "https://aussieexotics.com/",
                "script",
            )
            .unwrap(),
        );
        assert!(
            checked.matched,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception
        );
        assert!(checked.redirect.is_some(), "{:#?}", checked);
        // Check for the specific expected return script value in base64
        assert_eq!(checked.redirect.unwrap(), "data:application/javascript;base64,LyoqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioKCiAgICB1QmxvY2sgT3JpZ2luIC0gYSBicm93c2VyIGV4dGVuc2lvbiB0byBibG9jayByZXF1ZXN0cy4KICAgIENvcHlyaWdodCAoQykgMjAxOS1wcmVzZW50IFJheW1vbmQgSGlsbAoKICAgIFRoaXMgcHJvZ3JhbSBpcyBmcmVlIHNvZnR3YXJlOiB5b3UgY2FuIHJlZGlzdHJpYnV0ZSBpdCBhbmQvb3IgbW9kaWZ5CiAgICBpdCB1bmRlciB0aGUgdGVybXMgb2YgdGhlIEdOVSBHZW5lcmFsIFB1YmxpYyBMaWNlbnNlIGFzIHB1Ymxpc2hlZCBieQogICAgdGhlIEZyZWUgU29mdHdhcmUgRm91bmRhdGlvbiwgZWl0aGVyIHZlcnNpb24gMyBvZiB0aGUgTGljZW5zZSwgb3IKICAgIChhdCB5b3VyIG9wdGlvbikgYW55IGxhdGVyIHZlcnNpb24uCgogICAgVGhpcyBwcm9ncmFtIGlzIGRpc3RyaWJ1dGVkIGluIHRoZSBob3BlIHRoYXQgaXQgd2lsbCBiZSB1c2VmdWwsCiAgICBidXQgV0lUSE9VVCBBTlkgV0FSUkFOVFk7IHdpdGhvdXQgZXZlbiB0aGUgaW1wbGllZCB3YXJyYW50eSBvZgogICAgTUVSQ0hBTlRBQklMSVRZIG9yIEZJVE5FU1MgRk9SIEEgUEFSVElDVUxBUiBQVVJQT1NFLiAgU2VlIHRoZQogICAgR05VIEdlbmVyYWwgUHVibGljIExpY2Vuc2UgZm9yIG1vcmUgZGV0YWlscy4KCiAgICBZb3Ugc2hvdWxkIGhhdmUgcmVjZWl2ZWQgYSBjb3B5IG9mIHRoZSBHTlUgR2VuZXJhbCBQdWJsaWMgTGljZW5zZQogICAgYWxvbmcgd2l0aCB0aGlzIHByb2dyYW0uICBJZiBub3QsIHNlZSB7aHR0cDovL3d3dy5nbnUub3JnL2xpY2Vuc2VzL30uCgogICAgSG9tZTogaHR0cHM6Ly9naXRodWIuY29tL2dvcmhpbGwvdUJsb2NrCiovCgooZnVuY3Rpb24oKSB7CiAgICAndXNlIHN0cmljdCc7CiAgICBpZiAoIGFtem5hZHMgKSB7CiAgICAgICAgcmV0dXJuOwogICAgfQogICAgdmFyIHcgPSB3aW5kb3c7CiAgICB2YXIgbm9vcGZuID0gZnVuY3Rpb24oKSB7CiAgICAgICAgOwogICAgfS5iaW5kKCk7CiAgICB2YXIgYW16bmFkcyA9IHsKICAgICAgICBhcHBlbmRTY3JpcHRUYWc6IG5vb3BmbiwKICAgICAgICBhcHBlbmRUYXJnZXRpbmdUb0FkU2VydmVyVXJsOiBub29wZm4sCiAgICAgICAgYXBwZW5kVGFyZ2V0aW5nVG9RdWVyeVN0cmluZzogbm9vcGZuLAogICAgICAgIGNsZWFyVGFyZ2V0aW5nRnJvbUdQVEFzeW5jOiBub29wZm4sCiAgICAgICAgZG9BbGxUYXNrczogbm9vcGZuLAogICAgICAgIGRvR2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICBkb1Rhc2s6IG5vb3BmbiwKICAgICAgICBkZXRlY3RJZnJhbWVBbmRHZXRVUkw6IG5vb3BmbiwKICAgICAgICBnZXRBZHM6IG5vb3BmbiwKICAgICAgICBnZXRBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldEFkRm9yU2xvdDogbm9vcGZuLAogICAgICAgIGdldEFkc0NhbGxiYWNrOiBub29wZm4sCiAgICAgICAgZ2V0RGlzcGxheUFkczogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGdldEtleXM6IG5vb3BmbiwKICAgICAgICBnZXRSZWZlcnJlclVSTDogbm9vcGZuLAogICAgICAgIGdldFNjcmlwdFNvdXJjZTogbm9vcGZuLAogICAgICAgIGdldFRhcmdldGluZzogbm9vcGZuLAogICAgICAgIGdldFRva2Vuczogbm9vcGZuLAogICAgICAgIGdldFZhbGlkTWlsbGlzZWNvbmRzOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHM6IG5vb3BmbiwKICAgICAgICBnZXRWaWRlb0Fkc0FzeW5jOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGhhbmRsZUNhbGxCYWNrOiBub29wZm4sCiAgICAgICAgaGFzQWRzOiBub29wZm4sCiAgICAgICAgcmVuZGVyQWQ6IG5vb3BmbiwKICAgICAgICBzYXZlQWRzOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nRm9yR1BUQXN5bmM6IG5vb3BmbiwKICAgICAgICBzZXRUYXJnZXRpbmdGb3JHUFRTeW5jOiBub29wZm4sCiAgICAgICAgdHJ5R2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICB1cGRhdGVBZHM6IG5vb3BmbgogICAgfTsKICAgIHcuYW16bmFkcyA9IGFtem5hZHM7CiAgICB3LmFtem5fYWRzID0gdy5hbXpuX2FkcyB8fCBub29wZm47CiAgICB3LmFheF93cml0ZSA9IHcuYWF4X3dyaXRlIHx8IG5vb3BmbjsKICAgIHcuYWF4X3JlbmRlcl9hZCA9IHcuYWF4X3JlbmRlcl9hZCB8fCBub29wZm47Cn0pKCk7Cg==");
    }
    {
        let checked = engine.check_network_request(
            &Request::new(
                "https://www.googletagservices.com/tag/js/gpt.js",
                "https://tvguide.com/",
                "script",
            )
            .unwrap(),
        );
        assert!(
            checked.matched,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception
        );
        assert!(checked.redirect.is_some(), "{:#?}", checked);
        assert_eq!(checked.redirect.unwrap(), "data:application/javascript;base64,LyoqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioKCiAgICB1QmxvY2sgT3JpZ2luIC0gYSBicm93c2VyIGV4dGVuc2lvbiB0byBibG9jayByZXF1ZXN0cy4KICAgIENvcHlyaWdodCAoQykgMjAxOS1wcmVzZW50IFJheW1vbmQgSGlsbAoKICAgIFRoaXMgcHJvZ3JhbSBpcyBmcmVlIHNvZnR3YXJlOiB5b3UgY2FuIHJlZGlzdHJpYnV0ZSBpdCBhbmQvb3IgbW9kaWZ5CiAgICBpdCB1bmRlciB0aGUgdGVybXMgb2YgdGhlIEdOVSBHZW5lcmFsIFB1YmxpYyBMaWNlbnNlIGFzIHB1Ymxpc2hlZCBieQogICAgdGhlIEZyZWUgU29mdHdhcmUgRm91bmRhdGlvbiwgZWl0aGVyIHZlcnNpb24gMyBvZiB0aGUgTGljZW5zZSwgb3IKICAgIChhdCB5b3VyIG9wdGlvbikgYW55IGxhdGVyIHZlcnNpb24uCgogICAgVGhpcyBwcm9ncmFtIGlzIGRpc3RyaWJ1dGVkIGluIHRoZSBob3BlIHRoYXQgaXQgd2lsbCBiZSB1c2VmdWwsCiAgICBidXQgV0lUSE9VVCBBTlkgV0FSUkFOVFk7IHdpdGhvdXQgZXZlbiB0aGUgaW1wbGllZCB3YXJyYW50eSBvZgogICAgTUVSQ0hBTlRBQklMSVRZIG9yIEZJVE5FU1MgRk9SIEEgUEFSVElDVUxBUiBQVVJQT1NFLiAgU2VlIHRoZQogICAgR05VIEdlbmVyYWwgUHVibGljIExpY2Vuc2UgZm9yIG1vcmUgZGV0YWlscy4KCiAgICBZb3Ugc2hvdWxkIGhhdmUgcmVjZWl2ZWQgYSBjb3B5IG9mIHRoZSBHTlUgR2VuZXJhbCBQdWJsaWMgTGljZW5zZQogICAgYWxvbmcgd2l0aCB0aGlzIHByb2dyYW0uICBJZiBub3QsIHNlZSB7aHR0cDovL3d3dy5nbnUub3JnL2xpY2Vuc2VzL30uCgogICAgSG9tZTogaHR0cHM6Ly9naXRodWIuY29tL2dvcmhpbGwvdUJsb2NrCiovCgooZnVuY3Rpb24oKSB7CiAgICAndXNlIHN0cmljdCc7CiAgICAvLyBodHRwczovL2RldmVsb3BlcnMuZ29vZ2xlLmNvbS9kb3VibGVjbGljay1ncHQvcmVmZXJlbmNlCiAgICBjb25zdCBub29wZm4gPSBmdW5jdGlvbigpIHsKICAgIH0uYmluZCgpOwogICAgY29uc3Qgbm9vcHRoaXNmbiA9IGZ1bmN0aW9uKCkgewogICAgICAgIHJldHVybiB0aGlzOwogICAgfTsKICAgIGNvbnN0IG5vb3BudWxsZm4gPSBmdW5jdGlvbigpIHsKICAgICAgICByZXR1cm4gbnVsbDsKICAgIH07CiAgICBjb25zdCBub29wYXJyYXlmbiA9IGZ1bmN0aW9uKCkgewogICAgICAgIHJldHVybiBbXTsKICAgIH07CiAgICBjb25zdCBub29wc3RyZm4gPSBmdW5jdGlvbigpIHsKICAgICAgICByZXR1cm4gJyc7CiAgICB9OwogICAgLy8KICAgIGNvbnN0IGNvbXBhbmlvbkFkc1NlcnZpY2UgPSB7CiAgICAgICAgYWRkRXZlbnRMaXN0ZW5lcjogbm9vcHRoaXNmbiwKICAgICAgICBlbmFibGVTeW5jTG9hZGluZzogbm9vcGZuLAogICAgICAgIHNldFJlZnJlc2hVbmZpbGxlZFNsb3RzOiBub29wZm4KICAgIH07CiAgICBjb25zdCBjb250ZW50U2VydmljZSA9IHsKICAgICAgICBhZGRFdmVudExpc3RlbmVyOiBub29wdGhpc2ZuLAogICAgICAgIHNldENvbnRlbnQ6IG5vb3BmbgogICAgfTsKICAgIGNvbnN0IFBhc3NiYWNrU2xvdCA9IGZ1bmN0aW9uKCkgewogICAgfTsKICAgIGxldCBwID0gUGFzc2JhY2tTbG90LnByb3RvdHlwZTsKICAgIHAuZGlzcGxheSA9IG5vb3BmbjsKICAgIHAuZ2V0ID0gbm9vcG51bGxmbjsKICAgIHAuc2V0ID0gbm9vcHRoaXNmbjsKICAgIHAuc2V0Q2xpY2tVcmwgPSBub29wdGhpc2ZuOwogICAgcC5zZXRUYWdGb3JDaGlsZERpcmVjdGVkVHJlYXRtZW50ID0gbm9vcHRoaXNmbjsKICAgIHAuc2V0VGFyZ2V0aW5nID0gbm9vcHRoaXNmbjsKICAgIHAudXBkYXRlVGFyZ2V0aW5nRnJvbU1hcCA9IG5vb3B0aGlzZm47CiAgICBjb25zdCBwdWJBZHNTZXJ2aWNlID0gewogICAgICAgIGFkZEV2ZW50TGlzdGVuZXI6IG5vb3B0aGlzZm4sCiAgICAgICAgY2xlYXI6IG5vb3BmbiwKICAgICAgICBjbGVhckNhdGVnb3J5RXhjbHVzaW9uczogbm9vcHRoaXNmbiwKICAgICAgICBjbGVhclRhZ0ZvckNoaWxkRGlyZWN0ZWRUcmVhdG1lbnQ6IG5vb3B0aGlzZm4sCiAgICAgICAgY2xlYXJUYXJnZXRpbmc6IG5vb3B0aGlzZm4sCiAgICAgICAgY29sbGFwc2VFbXB0eURpdnM6IG5vb3BmbiwKICAgICAgICBkZWZpbmVPdXRPZlBhZ2VQYXNzYmFjazogZnVuY3Rpb24oKSB7IHJldHVybiBuZXcgUGFzc2JhY2tTbG90KCk7IH0sCiAgICAgICAgZGVmaW5lUGFzc2JhY2s6IGZ1bmN0aW9uKCkgeyByZXR1cm4gbmV3IFBhc3NiYWNrU2xvdCgpOyB9LAogICAgICAgIGRpc2FibGVJbml0aWFsTG9hZDogbm9vcGZuLAogICAgICAgIGRpc3BsYXk6IG5vb3BmbiwKICAgICAgICBlbmFibGVBc3luY1JlbmRlcmluZzogbm9vcGZuLAogICAgICAgIGVuYWJsZVNpbmdsZVJlcXVlc3Q6IG5vb3BmbiwKICAgICAgICBlbmFibGVTeW5jUmVuZGVyaW5nOiBub29wZm4sCiAgICAgICAgZW5hYmxlVmlkZW9BZHM6IG5vb3BmbiwKICAgICAgICBnZXQ6IG5vb3BudWxsZm4sCiAgICAgICAgZ2V0QXR0cmlidXRlS2V5czogbm9vcGFycmF5Zm4sCiAgICAgICAgZ2V0VGFyZ2V0aW5nOiBub29wZm4sCiAgICAgICAgZ2V0VGFyZ2V0aW5nS2V5czogbm9vcGFycmF5Zm4sCiAgICAgICAgZ2V0U2xvdHM6IG5vb3BhcnJheWZuLAogICAgICAgIHJlZnJlc2g6IG5vb3BmbiwKICAgICAgICByZW1vdmVFdmVudExpc3RlbmVyOiBub29wZm4sCiAgICAgICAgc2V0OiBub29wdGhpc2ZuLAogICAgICAgIHNldENhdGVnb3J5RXhjbHVzaW9uOiBub29wdGhpc2ZuLAogICAgICAgIHNldENlbnRlcmluZzogbm9vcGZuLAogICAgICAgIHNldENvb2tpZU9wdGlvbnM6IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0Rm9yY2VTYWZlRnJhbWU6IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0TG9jYXRpb246IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0UHVibGlzaGVyUHJvdmlkZWRJZDogbm9vcHRoaXNmbiwKICAgICAgICBzZXRQcml2YWN5U2V0dGluZ3M6IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0UmVxdWVzdE5vblBlcnNvbmFsaXplZEFkczogbm9vcHRoaXNmbiwKICAgICAgICBzZXRTYWZlRnJhbWVDb25maWc6IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0VGFnRm9yQ2hpbGREaXJlY3RlZFRyZWF0bWVudDogbm9vcHRoaXNmbiwKICAgICAgICBzZXRUYXJnZXRpbmc6IG5vb3B0aGlzZm4sCiAgICAgICAgc2V0VmlkZW9Db250ZW50OiBub29wdGhpc2ZuLAogICAgICAgIHVwZGF0ZUNvcnJlbGF0b3I6IG5vb3BmbgogICAgfTsKICAgIGNvbnN0IFNpemVNYXBwaW5nQnVpbGRlciA9IGZ1bmN0aW9uKCkgewogICAgfTsKICAgIHAgPSBTaXplTWFwcGluZ0J1aWxkZXIucHJvdG90eXBlOwogICAgcC5hZGRTaXplID0gbm9vcHRoaXNmbjsKICAgIHAuYnVpbGQgPSBub29wbnVsbGZuOwogICAgY29uc3QgU2xvdCA9IGZ1bmN0aW9uKCkgewogICAgfTsKICAgIHAgPSBTbG90LnByb3RvdHlwZTsKICAgIHAuYWRkU2VydmljZSA9IG5vb3B0aGlzZm47CiAgICBwLmNsZWFyQ2F0ZWdvcnlFeGNsdXNpb25zID0gbm9vcHRoaXNmbjsKICAgIHAuY2xlYXJUYXJnZXRpbmcgPSBub29wdGhpc2ZuOwogICAgcC5kZWZpbmVTaXplTWFwcGluZyA9IG5vb3B0aGlzZm47CiAgICBwLmdldCA9IG5vb3BudWxsZm47CiAgICBwLmdldEFkVW5pdFBhdGggPSBub29wYXJyYXlmbjsKICAgIHAuZ2V0QXR0cmlidXRlS2V5cyA9IG5vb3BhcnJheWZuOwogICAgcC5nZXRDYXRlZ29yeUV4Y2x1c2lvbnMgPSBub29wYXJyYXlmbjsKICAgIHAuZ2V0RG9tSWQgPSBub29wc3RyZm47CiAgICBwLmdldFJlc3BvbnNlSW5mb3JtYXRpb24gPSBub29wbnVsbGZuOwogICAgcC5nZXRTbG90RWxlbWVudElkID0gbm9vcHN0cmZuOwogICAgcC5nZXRTbG90SWQgPSBub29wdGhpc2ZuOwogICAgcC5nZXRUYXJnZXRpbmcgPSBub29wYXJyYXlmbjsKICAgIHAuZ2V0VGFyZ2V0aW5nS2V5cyA9IG5vb3BhcnJheWZuOwogICAgcC5zZXQgPSBub29wdGhpc2ZuOwogICAgcC5zZXRDYXRlZ29yeUV4Y2x1c2lvbiA9IG5vb3B0aGlzZm47CiAgICBwLnNldENsaWNrVXJsID0gbm9vcHRoaXNmbjsKICAgIHAuc2V0Q29sbGFwc2VFbXB0eURpdiA9IG5vb3B0aGlzZm47CiAgICBwLnNldFRhcmdldGluZyA9IG5vb3B0aGlzZm47CiAgICBwLnVwZGF0ZVRhcmdldGluZ0Zyb21NYXAgPSBub29wdGhpc2ZuOwogICAgLy8KICAgIGNvbnN0IGdwdCA9IHdpbmRvdy5nb29nbGV0YWcgfHwge307CiAgICBjb25zdCBjbWQgPSBncHQuY21kIHx8IFtdOwogICAgZ3B0LmFwaVJlYWR5ID0gdHJ1ZTsKICAgIGdwdC5jbWQgPSBbXTsKICAgIGdwdC5jbWQucHVzaCA9IGZ1bmN0aW9uKGEpIHsKICAgICAgICB0cnkgewogICAgICAgICAgICBhKCk7CiAgICAgICAgfSBjYXRjaCAoZXgpIHsKICAgICAgICB9CiAgICAgICAgcmV0dXJuIDE7CiAgICB9OwogICAgZ3B0LmNvbXBhbmlvbkFkcyA9IGZ1bmN0aW9uKCkgeyByZXR1cm4gY29tcGFuaW9uQWRzU2VydmljZTsgfTsKICAgIGdwdC5jb250ZW50ID0gZnVuY3Rpb24oKSB7IHJldHVybiBjb250ZW50U2VydmljZTsgfTsKICAgIGdwdC5kZWZpbmVPdXRPZlBhZ2VTbG90ID0gZnVuY3Rpb24oKSB7IHJldHVybiBuZXcgU2xvdCgpOyB9OwogICAgZ3B0LmRlZmluZVNsb3QgPSBmdW5jdGlvbigpIHsgcmV0dXJuIG5ldyBTbG90KCk7IH07CiAgICBncHQuZGVzdHJveVNsb3RzID0gbm9vcGZuOwogICAgZ3B0LmRpc2FibGVQdWJsaXNoZXJDb25zb2xlID0gbm9vcGZuOwogICAgZ3B0LmRpc3BsYXkgPSBub29wZm47CiAgICBncHQuZW5hYmxlU2VydmljZXMgPSBub29wZm47CiAgICBncHQuZ2V0VmVyc2lvbiA9IG5vb3BzdHJmbjsKICAgIGdwdC5wdWJhZHMgPSBmdW5jdGlvbigpIHsgcmV0dXJuIHB1YkFkc1NlcnZpY2U7IH07CiAgICBncHQucHViYWRzUmVhZHkgPSB0cnVlOwogICAgZ3B0LnNldEFkSWZyYW1lVGl0bGUgPSBub29wZm47CiAgICBncHQuc2l6ZU1hcHBpbmcgPSBmdW5jdGlvbigpIHsgcmV0dXJuIG5ldyBTaXplTWFwcGluZ0J1aWxkZXIoKTsgfTsKICAgIHdpbmRvdy5nb29nbGV0YWcgPSBncHQ7CiAgICB3aGlsZSAoIGNtZC5sZW5ndGggIT09IDAgKSB7CiAgICAgICAgZ3B0LmNtZC5wdXNoKGNtZC5zaGlmdCgpKTsKICAgIH0KfSkoKTsK");
    }
}

#[test]
/// Ensure that one engine's serialization result can be exactly reproduced by another engine after
/// deserializing from it.
fn stable_serialization_through_load() {
    let engine1 = Engine::from_filter_set(ALL_FILTERS.lock().unwrap().clone(), true);
    let ser1 = engine1.serialize().unwrap();

    let mut engine2 = Engine::default();
    engine2.deserialize(&ser1).unwrap();
    let ser2 = engine2.serialize().unwrap();

    assert_eq!(ser1, ser2);
}
