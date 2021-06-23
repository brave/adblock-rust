//! Contains methods useful for building `Resource` descriptors from resources directly from files
//! in the uBlock Origin repository.

use regex::Regex;
use once_cell::sync::Lazy;
use std::io::Read;
use std::fs::File;
use std::path::Path;
use crate::resources::{Resource, ResourceType, MimeType};

static TOP_COMMENT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^/\*[\S\s]+?\n\*/\s*"#).unwrap());
static NON_EMPTY_LINE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"\S"#).unwrap());

/// Represents a single entry of the `redirectableResources` map from uBlock Origin's
/// `redirect-engine.js`.
///
/// - `name` is the name of a resource, corresponding to its path in the `web_accessible_resources`
/// directory
///
/// - `alias` is an optional additional name that can be used to reference the resource
///
/// - `data` is either `"text"` or `"blob"`, but is currently unused in `adblock-rust`. Within
/// uBlock Origin, it's used to prevent text files from being encoded in base64 in a data URL.
struct ResourceProperties {
    name: String,
    alias: Option<String>,
    data: Option<String>,
}

/// Directly deserializable representation of a resource's properties from `redirect-engine.js`.
#[derive(serde::Deserialize)]
struct JsResourceProperties {
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    data: Option<String>,
    #[serde(default)]
    params: Option<Vec<String>>,
}

/// Maps the name of the resource to its properties in a 2-element tuple.
type JsResourceEntry = (String, JsResourceProperties);

const REDIRECTABLE_RESOURCES_DECLARATION: &str = "const redirectableResources = new Map([";
//  ]);
static MAP_END_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s*\]\s*\)"#).unwrap());

static TRAILING_COMMA_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#",([\],\}])"#).unwrap());
static UNQUOTED_FIELD_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"([\{,])([a-zA-Z][a-zA-Z0-9_]*):"#).unwrap());

/// Reads data from a a file in the format of uBlock Origin's `redirect-engine.js` file to
/// determine the files in the `web_accessible_resources` directory, as well as any of their
/// aliases.
///
/// This is read from the `redirectableResources` map.
fn read_redirectable_resource_mapping(mapfile_data: &str) -> Vec<ResourceProperties> {
    // This isn't bulletproof, but it should handle the historical versions of the mapping
    // correctly, and having a strict JSON parser should catch any unexpected format changes. Plus,
    // it prevents dependending on a full JS engine.

    // Extract just the map. It's between REDIRECTABLE_RESOURCES_DECLARATION and MAP_END_RE.
    let mut map: String = mapfile_data
        .lines()
        .skip_while(|line| *line != REDIRECTABLE_RESOURCES_DECLARATION)
        .take_while(|line| !MAP_END_RE.is_match(line))
        // Strip any trailing comments from each line.
        .map(|line| if let Some(i) = line.find("//") {
            &line[..i]
        } else {
            line
        })
        // Remove all newlines from the entire string.
        .fold(String::new(), |s, line| s + line);

    // Add back the final square brace that was omitted above as part of MAP_END_RE.
    map.push(']');

    // Trim out the beginning `const redirectableResources = new Map(`.
    // Also, replace all single quote characters with double quotes.
    assert!(map.starts_with(REDIRECTABLE_RESOURCES_DECLARATION));
    map = map[REDIRECTABLE_RESOURCES_DECLARATION.len()-1..].replace('\'', "\"");

    // Remove all whitespace from the entire string.
    map.retain(|c| !c.is_whitespace());

    // Replace all matches for `,]` or `,}` with `]` or `}`, respectively.
    map = TRAILING_COMMA_RE.replace_all(&map, |caps: &regex::Captures| caps[1].to_string()).to_string();

    // Replace all property keys directly preceded by a `{` or a `,` and followed by a `:` with
    // double-quoted versions.
    map = UNQUOTED_FIELD_RE.replace_all(&map, |caps: &regex::Captures| format!("{}\"{}\":", &caps[1], &caps[2])).to_string();

    // It *should* be valid JSON now, so parse it with serde_json.
    let parsed: Vec<JsResourceEntry> = serde_json::from_str(&map).unwrap();

    parsed.into_iter().filter_map(|(name, props)| {
        // Ignore resources with params for now, since there's no support for them currently.
        if props.params.is_some() {
            None
        } else {
            Some(ResourceProperties {
                name,
                alias: props.alias,
                data: props.data,
            })
        }
    }).collect()
}

/// Reads data from a file in the form of uBlock Origin's `scriptlets.js` file and produces
/// templatable scriptlets for use in cosmetic filtering.
fn read_template_resources(scriptlets_data: &str) -> Vec<Resource> {
    let mut resources = Vec::new();

    let uncommented = TOP_COMMENT_RE.replace_all(&scriptlets_data, "");
    let mut name: Option<&str> = None;
    let mut details = std::collections::HashMap::new();
    let mut script = String::new();

    for line in uncommented.lines() {
        if line.starts_with('#') || line.starts_with("// ") {
            continue;
        }

        if name.is_none() {
            if line.starts_with("/// ") {
                name = Some(line[4..].trim());
            }
            continue;
        }

        if line.starts_with("/// ") {
            let mut line = line[4..].split_whitespace();
            let prop = line.next().expect("Detail line has property name");
            let value = line.next().expect("Detail line has property value");
            details.insert(prop, value);
            continue;
        }

        if NON_EMPTY_LINE_RE.is_match(line) {
            script += line.trim();
            continue;
        }

        let kind = if script.contains("{{1}}") {
            ResourceType::Template
        } else {
            ResourceType::Mime(MimeType::ApplicationJavascript)
        };
        resources.push(Resource {
            name: name.expect("Resource name must be specified").to_owned(),
            aliases: details.get("alias").iter().map(|alias| alias.to_string()).collect(),
            kind,
            content: base64::encode(&script),
        });

        name = None;
        details.clear();
        script.clear();
    }

    resources
}

/// Reads byte data from an arbitrary resource file, and assembles a `Resource` from it with the
/// provided `resource_info`.
fn build_resource_from_file_contents(resource_contents: &[u8], resource_info: &ResourceProperties) -> Resource {
    let name = resource_info.name.to_owned();
    let aliases = resource_info.alias.iter().map(|alias| alias.to_string()).collect();
    let mimetype = MimeType::from_extension(&resource_info.name[..]);
    let content = match mimetype {
        MimeType::ApplicationJavascript | MimeType::TextHtml | MimeType::TextPlain => {
            let utf8string = std::str::from_utf8(resource_contents).unwrap();
            base64::encode(&utf8string.replace('\r', ""))
        }
        _ => {
            base64::encode(&resource_contents)
        }
    };

    Resource {
        name,
        aliases,
        kind: ResourceType::Mime(mimetype),
        content,
    }
}

/// Produces a `Resource` from the `web_accessible_resource_dir` directory according to the
/// information in `resource_info.
fn read_resource_from_web_accessible_dir(web_accessible_resource_dir: &Path, resource_info: &ResourceProperties) -> Resource {
    let resource_path = web_accessible_resource_dir.join(&resource_info.name);
    if !resource_path.is_file() {
        panic!("Expected {:?} to be a file", resource_path);
    }
    let mut resource_file = File::open(resource_path).expect("open resource file for reading");
    let mut resource_contents = Vec::new();
    resource_file.read_to_end(&mut resource_contents).expect("read resource file contents");

    build_resource_from_file_contents(&resource_contents, resource_info)
}

/// Builds a `Vec` of `Resource`s from the specified paths on the filesystem:
///
/// - `web_accessible_resource_dir`: A folder full of resource files
///
/// - `redirect_engine_path`: A file in the format of uBlock Origin's `redirect-engine.js`
/// containing an index of the resources in `web_accessible_resource_dir`
///
/// - `scriptlets_path`: A file in the format of uBlock Origin's `scriptlets.js` containing
/// templatable scriptlet files for use in cosmetic filtering
///
/// The resulting resources can be serialized into JSON using `serde_json`.
pub fn assemble_web_accessible_resources(web_accessible_resource_dir: &Path, redirect_engine_path: &Path) -> Vec<Resource> {
    let mapfile_data = std::fs::read_to_string(redirect_engine_path).expect("read aliases path");
    let resource_properties = read_redirectable_resource_mapping(&mapfile_data);

    resource_properties.iter().map(|resource_info| {
        read_resource_from_web_accessible_dir(web_accessible_resource_dir, resource_info)
    }).collect()
}

pub fn assemble_scriptlet_resources(scriptlets_path: &Path) -> Vec<Resource> {
    let scriptlets_data = std::fs::read_to_string(scriptlets_path).expect("read scriptlets path");
    read_template_resources(&scriptlets_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_war_resource_assembly2() {
        let web_accessible_resource_dir = Path::new("data/test/fake-uBO-files/web_accessible_resources");
        let redirect_engine_path = Path::new("data/test/fake-uBO-files/redirect-engine2.js");
        let resources = assemble_web_accessible_resources(web_accessible_resource_dir, redirect_engine_path);

        let expected_resource_names = vec![
            "1x1.gif",
            "2x2.png",
            "3x2.png",
            "32x32.png",
            "addthis_widget.js",
            "amazon_ads.js",
            "amazon_apstag.js",
            "ampproject_v0.js",
            "chartbeat.js",
            //"click-to-load.html" is ignored because it has a params field.
            "doubleclick_instream_ad_status.js",
            "empty",
            "google-analytics_analytics.js",
            "google-analytics_cx_api.js",
            "google-analytics_ga.js",
            "google-analytics_inpage_linkid.js",
            "googlesyndication_adsbygoogle.js",
            "googletagmanager_gtm.js",
            "googletagservices_gpt.js",
            "hd-main.js",
            "ligatus_angular-tag.js",
            "monkeybroker.js",
            "noeval.js",
            "noeval-silent.js",
            "nobab.js",
            "nofab.js",
            "noop-0.1s.mp3",
            "noop-1s.mp4",
            "noop.html",
            "noop.js",
            "noop.txt",
            "outbrain-widget.js",
            "popads.js",
            "popads-dummy.js",
            "scorecardresearch_beacon.js",
            "window.open-defuser.js",
        ];

        for name in expected_resource_names {
            dbg!(&name);
            assert!(resources.iter()
                .find(|resource| {
                    if let ResourceType::Mime(_) = resource.kind {
                        resource.name == name
                    } else {
                        false
                    }
                }).is_some(), "{:?}", name);
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> = serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "1x1.gif");
        assert_eq!(reserialized[0].aliases, vec!["1x1-transparent.gif"]);
        assert_eq!(reserialized[0].kind, ResourceType::Mime(MimeType::ImageGif));

        assert_eq!(reserialized[28].name, "noop.js");
        assert_eq!(reserialized[28].aliases, vec!["noopjs"]);
        assert_eq!(reserialized[28].kind, ResourceType::Mime(MimeType::ApplicationJavascript));
        let noopjs_contents = std::fs::read_to_string(Path::new("data/test/fake-uBO-files/web_accessible_resources/noop.js")).unwrap().replace('\r', "");
        assert_eq!(
            std::str::from_utf8(
                &base64::decode(&reserialized[28].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            noopjs_contents,
        );
    }

    #[test]
    fn test_war_resource_assembly() {
        let web_accessible_resource_dir = Path::new("data/test/fake-uBO-files/web_accessible_resources");
        let redirect_engine_path = Path::new("data/test/fake-uBO-files/redirect-engine.js");
        let resources = assemble_web_accessible_resources(web_accessible_resource_dir, redirect_engine_path);

        let expected_resource_names = vec![
            "1x1.gif",
            "2x2.png",
            "3x2.png",
            "32x32.png",
            "addthis_widget.js",
            "amazon_ads.js",
            "ampproject_v0.js",
            "chartbeat.js",
            "disqus_embed.js",
            "disqus_forums_embed.js",
            "doubleclick_instream_ad_status.js",
            "empty",
            "google-analytics_analytics.js",
            "google-analytics_cx_api.js",
            "google-analytics_ga.js",
            "google-analytics_inpage_linkid.js",
            "googlesyndication_adsbygoogle.js",
            "googletagmanager_gtm.js",
            "googletagservices_gpt.js",
            "hd-main.js",
            "ligatus_angular-tag.js",
            "monkeybroker.js",
            "noeval.js",
            "noeval-silent.js",
            "nobab.js",
            "nofab.js",
            "noop-0.1s.mp3",
            "noop-1s.mp4",
            "noop.html",
            "noop.js",
            "noop.txt",
            "outbrain-widget.js",
            "popads.js",
            "popads-dummy.js",
            "scorecardresearch_beacon.js",
            "window.open-defuser.js",
        ];

        for name in expected_resource_names {
            assert!(resources.iter()
                .find(|resource| {
                    if let ResourceType::Mime(_) = resource.kind {
                        resource.name == name
                    } else {
                        false
                    }
                }).is_some());
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> = serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "1x1.gif");
        assert_eq!(reserialized[0].aliases, vec!["1x1-transparent.gif"]);
        assert_eq!(reserialized[0].kind, ResourceType::Mime(MimeType::ImageGif));

        assert_eq!(reserialized[29].name, "noop.js");
        assert_eq!(reserialized[29].aliases, vec!["noopjs"]);
        assert_eq!(reserialized[29].kind, ResourceType::Mime(MimeType::ApplicationJavascript));
        let noopjs_contents = std::fs::read_to_string(Path::new("data/test/fake-uBO-files/web_accessible_resources/noop.js")).unwrap().replace('\r', "");
        assert_eq!(
            std::str::from_utf8(
                &base64::decode(&reserialized[29].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            noopjs_contents,
        );
    }

    #[test]
    fn test_scriptlet_resource_assembly() {
        let scriptlets_path = Path::new("data/test/fake-uBO-files/scriptlets.js");
        let resources = assemble_scriptlet_resources(scriptlets_path);

        let expected_resource_names = vec![
            "abort-current-inline-script.js",
            "abort-on-property-read.js",
            "abort-on-property-write.js",
            "addEventListener-defuser.js",
            "addEventListener-logger.js",
            "json-prune.js",
            "nano-setInterval-booster.js",
            "nano-setTimeout-booster.js",
            "noeval-if.js",
            "remove-attr.js",
            "requestAnimationFrame-if.js",
            "set-constant.js",
            "setInterval-defuser.js",
            "no-setInterval-if.js",
            "setTimeout-defuser.js",
            "no-setTimeout-if.js",
            "webrtc-if.js",
            "window.name-defuser",
            "overlay-buster.js",
            "alert-buster.js",
            "gpt-defuser.js",
            "nowebrtc.js",
            "golem.de.js",
            "upmanager-defuser.js",
            "smartadserver.com.js",
            "adfly-defuser.js",
            "disable-newtab-links.js",
            "damoh-defuser.js",
            "twitch-videoad.js",
            "fingerprint2.js",
            "cookie-remover.js",
        ];

        for name in expected_resource_names {
            assert!(resources.iter()
                .find(|resource| {
                    match resource.kind {
                        ResourceType::Template | ResourceType::Mime(MimeType::ApplicationJavascript) => resource.name == name,
                        _ => false,
                    }
                })
                .is_some(), "failed to find {}", name);
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> = serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "abort-current-inline-script.js");
        assert_eq!(reserialized[0].aliases, vec!["acis.js"]);
        assert_eq!(reserialized[0].kind, ResourceType::Template);

        assert_eq!(reserialized[18].name, "overlay-buster.js");
        assert_eq!(reserialized[18].aliases, Vec::<String>::new());
        assert_eq!(reserialized[18].kind, ResourceType::Mime(MimeType::ApplicationJavascript));
        assert_eq!(
            std::str::from_utf8(
                &base64::decode(&reserialized[18].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            "(function() {if ( window !== window.top ) {return;}var tstart;var ttl = 30000;var delay = 0;var delayStep = 50;var buster = function() {var docEl = document.documentElement,bodyEl = document.body,vw = Math.min(docEl.clientWidth, window.innerWidth),vh = Math.min(docEl.clientHeight, window.innerHeight),tol = Math.min(vw, vh) * 0.05,el = document.elementFromPoint(vw/2, vh/2),style, rect;for (;;) {if ( el === null || el.parentNode === null || el === bodyEl ) {break;}style = window.getComputedStyle(el);if ( parseInt(style.zIndex, 10) >= 1000 || style.position === \'fixed\' ) {rect = el.getBoundingClientRect();if ( rect.left <= tol && rect.top <= tol && (vw - rect.right) <= tol && (vh - rect.bottom) < tol ) {el.parentNode.removeChild(el);tstart = Date.now();el = document.elementFromPoint(vw/2, vh/2);bodyEl.style.setProperty(\'overflow\', \'auto\', \'important\');docEl.style.setProperty(\'overflow\', \'auto\', \'important\');continue;}}el = el.parentNode;}if ( (Date.now() - tstart) < ttl ) {delay = Math.min(delay + delayStep, 1000);setTimeout(buster, delay);}};var domReady = function(ev) {if ( ev ) {document.removeEventListener(ev.type, domReady);}tstart = Date.now();setTimeout(buster, delay);};if ( document.readyState === \'loading\' ) {document.addEventListener(\'DOMContentLoaded\', domReady);} else {domReady();}})();",
        );
    }
}
