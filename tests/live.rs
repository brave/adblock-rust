extern crate adblock;
extern crate reqwest;

use adblock::blocker::{Blocker, BlockerOptions};
use adblock::engine::Engine;
use adblock::filters::network::NetworkFilter;
use adblock::resource_assembler::assemble_web_accessible_resources;

use serde::{Deserialize};
use std::fs::File;
use std::io::BufReader;

use csv;

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
            reqs.push(record);
        } else {
            println!("Could not parse {:?}", result);
        }
    }

    reqs
}

fn get_blocker_engine() -> Engine {
  let network_filters: Vec<NetworkFilter> = adblock::filter_lists::default::default_lists()
        .iter()
        .map(|list| {
            let filters: Vec<String> = reqwest::get(&list.url).expect("Could not request rules")
                .text().expect("Could not get rules as text")
                .lines()
                .map(|s| s.to_owned())
                .collect();

            let (network_filters, _) = adblock::lists::parse_filters(&filters, true, false, true);
            network_filters
        })
        .flatten()
        .collect();

  let blocker_options = BlockerOptions {
    debug: true,
    enable_optimizations: false,
    load_cosmetic_filters: false,
    load_network_filters: true
  };
  
    let mut engine = Engine {
        blocker: Blocker::new(network_filters, &blocker_options)
    };

    engine.with_tags(&["fb-embeds", "twitter-embeds"]);

    engine
}

fn get_blocker_engine_deserialized() -> Engine {
    let dat_url = "https://adblock-data.s3.amazonaws.com/4/rs-ABPFilterParserData.dat";
    let mut dat: Vec<u8> = vec![];
    let mut resp = reqwest::get(dat_url).expect("Could not request rules");
    resp.copy_to(&mut dat).expect("Could not copy response to byte array");

    let mut engine = Engine::from_rules(&[]);
    engine.deserialize(&dat).expect("Deserialization failed");
    engine.with_tags(&["fb-embeds", "twitter-embeds"]);
    engine
}

#[test]
fn check_live_specific_urls() {
    let engine = get_blocker_engine();
    {
        let checked = engine.check_network_urls(
            "https://static.scroll.com/js/scroll.js",
            "https://www.theverge.com/",
            "script");
        assert_eq!(checked.matched, false,
            "Expected match, got filter {:?}, exception {:?}",
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
fn check_live_deserialized() {
    let engine = get_blocker_engine_deserialized();
    let requests = load_requests();
    
    for req in requests {
        let checked = engine.check_network_urls(&req.url, &req.sourceUrl, &req.r#type);
        assert_eq!(checked.matched, req.blocked,
            "Expected match {} for {} {} {}",
            req.blocked, req.url, req.sourceUrl, req.r#type);
    }
}

#[test]
fn check_live_redirects() {
    let mut engine = get_blocker_engine();
    let redirect_engine_path = std::path::Path::new("data/test/fake-uBO-files/redirect-engine.js");
    let war_dir = std::path::Path::new("data/test/fake-uBO-files/web_accessible_resources");
    let resources = assemble_web_accessible_resources(war_dir, redirect_engine_path);

    engine.with_resources(&resources);
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