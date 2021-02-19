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
#[derive(serde::Deserialize)]
pub struct RemoteFilterSource {
    pub uuid: String,
    pub url: String,
    pub title: String,
    pub format: adblock::lists::FilterFormat,
    pub support_url: String,
}

pub async fn get_all_filters() -> adblock::lists::FilterSet {
    use futures::FutureExt;

    const DEFAULT_LISTS_URL: &'static str = "https://raw.githubusercontent.com/brave/adblock-resources/master/filter_lists/default.json";

    let default_lists: Vec<RemoteFilterSource> = async {
        let body = reqwest::get(DEFAULT_LISTS_URL).await.unwrap().text().await.unwrap();
        serde_json::from_str(&body).unwrap()
    }.await;

    let filters_fut: Vec<_> = default_lists
        .iter()
        .map(|list| {
            reqwest::get(&list.url)
                .then(|resp| resp
                    .expect("Could not request rules")
                    .text()
                ).map(move |text| (
                        list.format,
                        text.expect("Could not get rules as text")
                    )
                )
        })
        .collect();

    let mut filter_set = adblock::lists::FilterSet::default();

    futures::future::join_all(filters_fut)
        .await
        .iter()
        .for_each(|(format, list)| {
            filter_set.add_filters(&list.lines().map(|s| s.to_owned()).collect::<Vec<_>>(), *format);
        });

    filter_set
}

fn get_blocker_engine() -> Engine {
    let async_runtime = Runtime::new().expect("Could not start Tokio runtime");
    let filter_set = async_runtime.block_on(get_all_filters());

    let mut engine = Engine::from_filter_set(filter_set, true);

    engine.use_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn get_blocker_engine_deserialized() -> Engine {
    use futures::FutureExt;
    let async_runtime = Runtime::new().expect("Could not start Tokio runtime");

    let dat_url = "https://adblock-data.s3.brave.com/4/rs-ABPFilterParserData.dat";
    let resp_bytes_fut = reqwest::get(dat_url)
        .map(|e| e.expect("Could not request rules"))
        .then(|resp| resp.bytes());
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
    
    let engine = Engine::from_rules_parametrised(&filters, adblock::lists::FilterFormat::Standard, true, false);
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
fn check_live_deserialized_specific_urls() {
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
fn check_live_deserialized_file() {
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

#[test]
#[ignore]
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

#[cfg(feature = "resource_assembler")]
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
        assert!(checked.redirect.is_some());
        // Check for the specific expected return script value in base64
        assert_eq!(checked.redirect.unwrap(), "data:application/javascript;base64,LyoqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKioKCiAgICB1QmxvY2sgT3JpZ2luIC0gYSBicm93c2VyIGV4dGVuc2lvbiB0byBibG9jayByZXF1ZXN0cy4KICAgIENvcHlyaWdodCAoQykgMjAxOS1wcmVzZW50IFJheW1vbmQgSGlsbAoKICAgIFRoaXMgcHJvZ3JhbSBpcyBmcmVlIHNvZnR3YXJlOiB5b3UgY2FuIHJlZGlzdHJpYnV0ZSBpdCBhbmQvb3IgbW9kaWZ5CiAgICBpdCB1bmRlciB0aGUgdGVybXMgb2YgdGhlIEdOVSBHZW5lcmFsIFB1YmxpYyBMaWNlbnNlIGFzIHB1Ymxpc2hlZCBieQogICAgdGhlIEZyZWUgU29mdHdhcmUgRm91bmRhdGlvbiwgZWl0aGVyIHZlcnNpb24gMyBvZiB0aGUgTGljZW5zZSwgb3IKICAgIChhdCB5b3VyIG9wdGlvbikgYW55IGxhdGVyIHZlcnNpb24uCgogICAgVGhpcyBwcm9ncmFtIGlzIGRpc3RyaWJ1dGVkIGluIHRoZSBob3BlIHRoYXQgaXQgd2lsbCBiZSB1c2VmdWwsCiAgICBidXQgV0lUSE9VVCBBTlkgV0FSUkFOVFk7IHdpdGhvdXQgZXZlbiB0aGUgaW1wbGllZCB3YXJyYW50eSBvZgogICAgTUVSQ0hBTlRBQklMSVRZIG9yIEZJVE5FU1MgRk9SIEEgUEFSVElDVUxBUiBQVVJQT1NFLiAgU2VlIHRoZQogICAgR05VIEdlbmVyYWwgUHVibGljIExpY2Vuc2UgZm9yIG1vcmUgZGV0YWlscy4KCiAgICBZb3Ugc2hvdWxkIGhhdmUgcmVjZWl2ZWQgYSBjb3B5IG9mIHRoZSBHTlUgR2VuZXJhbCBQdWJsaWMgTGljZW5zZQogICAgYWxvbmcgd2l0aCB0aGlzIHByb2dyYW0uICBJZiBub3QsIHNlZSB7aHR0cDovL3d3dy5nbnUub3JnL2xpY2Vuc2VzL30uCgogICAgSG9tZTogaHR0cHM6Ly9naXRodWIuY29tL2dvcmhpbGwvdUJsb2NrCiovCgooZnVuY3Rpb24oKSB7CiAgICAndXNlIHN0cmljdCc7CiAgICBpZiAoIGFtem5hZHMgKSB7CiAgICAgICAgcmV0dXJuOwogICAgfQogICAgdmFyIHcgPSB3aW5kb3c7CiAgICB2YXIgbm9vcGZuID0gZnVuY3Rpb24oKSB7CiAgICAgICAgOwogICAgfS5iaW5kKCk7CiAgICB2YXIgYW16bmFkcyA9IHsKICAgICAgICBhcHBlbmRTY3JpcHRUYWc6IG5vb3BmbiwKICAgICAgICBhcHBlbmRUYXJnZXRpbmdUb0FkU2VydmVyVXJsOiBub29wZm4sCiAgICAgICAgYXBwZW5kVGFyZ2V0aW5nVG9RdWVyeVN0cmluZzogbm9vcGZuLAogICAgICAgIGNsZWFyVGFyZ2V0aW5nRnJvbUdQVEFzeW5jOiBub29wZm4sCiAgICAgICAgZG9BbGxUYXNrczogbm9vcGZuLAogICAgICAgIGRvR2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICBkb1Rhc2s6IG5vb3BmbiwKICAgICAgICBkZXRlY3RJZnJhbWVBbmRHZXRVUkw6IG5vb3BmbiwKICAgICAgICBnZXRBZHM6IG5vb3BmbiwKICAgICAgICBnZXRBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldEFkRm9yU2xvdDogbm9vcGZuLAogICAgICAgIGdldEFkc0NhbGxiYWNrOiBub29wZm4sCiAgICAgICAgZ2V0RGlzcGxheUFkczogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNBc3luYzogbm9vcGZuLAogICAgICAgIGdldERpc3BsYXlBZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGdldEtleXM6IG5vb3BmbiwKICAgICAgICBnZXRSZWZlcnJlclVSTDogbm9vcGZuLAogICAgICAgIGdldFNjcmlwdFNvdXJjZTogbm9vcGZuLAogICAgICAgIGdldFRhcmdldGluZzogbm9vcGZuLAogICAgICAgIGdldFRva2Vuczogbm9vcGZuLAogICAgICAgIGdldFZhbGlkTWlsbGlzZWNvbmRzOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHM6IG5vb3BmbiwKICAgICAgICBnZXRWaWRlb0Fkc0FzeW5jOiBub29wZm4sCiAgICAgICAgZ2V0VmlkZW9BZHNDYWxsYmFjazogbm9vcGZuLAogICAgICAgIGhhbmRsZUNhbGxCYWNrOiBub29wZm4sCiAgICAgICAgaGFzQWRzOiBub29wZm4sCiAgICAgICAgcmVuZGVyQWQ6IG5vb3BmbiwKICAgICAgICBzYXZlQWRzOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nOiBub29wZm4sCiAgICAgICAgc2V0VGFyZ2V0aW5nRm9yR1BUQXN5bmM6IG5vb3BmbiwKICAgICAgICBzZXRUYXJnZXRpbmdGb3JHUFRTeW5jOiBub29wZm4sCiAgICAgICAgdHJ5R2V0QWRzQXN5bmM6IG5vb3BmbiwKICAgICAgICB1cGRhdGVBZHM6IG5vb3BmbgogICAgfTsKICAgIHcuYW16bmFkcyA9IGFtem5hZHM7CiAgICB3LmFtem5fYWRzID0gdy5hbXpuX2FkcyB8fCBub29wZm47CiAgICB3LmFheF93cml0ZSA9IHcuYWF4X3dyaXRlIHx8IG5vb3BmbjsKICAgIHcuYWF4X3JlbmRlcl9hZCA9IHcuYWF4X3JlbmRlcl9hZCB8fCBub29wZm47Cn0pKCk7Cg==")
    }
    {
        let checked = engine.check_network_urls(
            "https://www.googletagservices.com/tag/js/gpt.js",
            "https://winniethepooh.disney.com/",
            "script");
        assert_eq!(checked.matched, true,
            "Expected match, got filter {:?}, exception {:?}",
            checked.filter, checked.exception);
        assert!(checked.redirect.is_some());
    }
    
}
