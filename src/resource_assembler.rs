use std::io::Read;
use regex::Regex;
use std::fs::File;
use std::path::Path;
use serde::{Serialize, Deserialize};

lazy_static! {
    //    [ '1x1.gif', {
    static ref MAP_NAME_KEY_RE: Regex = Regex::new(r#"^\s*\[\s*'([a-zA-Z0-9\-_\.]+)',\s*\{"#).unwrap();
    //      alias: '1x1-transparent.gif',
    static ref MAP_PROPERTY_KEY_RE: Regex = Regex::new(r#"^\s*([a-zA-Z0-9_]+):\s*'([a-zA-Z0-9\-_\./\*]+)',"#).unwrap();
    //    } ],
    static ref MAP_END_KEY_RE: Regex = Regex::new(r#"^\s*\}\s*\]"#).unwrap();
    //  ]);
    static ref MAP_END_RE: Regex = Regex::new(r#"^\s*\]\s*\)"#).unwrap();

    static ref TEMPLATE_ARGUMENT_RE: Regex = Regex::new(r"\{\{\d\}\}").unwrap();
    static ref ESCAPE_SCRIPTLET_ARG_RE: Regex = Regex::new(r#"[\\'"]"#).unwrap();
    static ref TOP_COMMENT_RE: Regex = Regex::new(r#"^/\*[\S\s]+?\n\*/\s*"#).unwrap();
    static ref NON_EMPTY_LINE_RE: Regex = Regex::new(r#"\S"#).unwrap();
}

/// Struct representing a resource that can be used by an adblocking engine.
///
/// - `name`: Represents the primary name of the resource, often a filename
///
/// - `aliases`: Represents secondary names that can be used to access the resource
///
/// - `kind`: How to interpret the resource data within `content`
///
/// - `content`: The resource data, encoded using standard base64 configuration
#[derive(Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: ResourceType,
    pub content: String,
}

/// Different ways that the data within the `content` field of a `Resource` can be interpreted.
///
/// - `Mime(type)` - interpret the data according to the MIME type represented by `type`
///
/// - `Template` - interpret the data as a Javascript scriptlet template, with embedded template
/// parameters in the form of `{{1}}`, `{{2}}`, etc.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Mime(MimeType),
    Template,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(into = "String")]
#[serde(from = "&str")]
pub enum MimeType {
    ImageGif,
    TextHtml,
    ApplicationJavascript,
    AudioMp3,
    VideoMp4,
    ImagePng,
    TextPlain,
    Unknown,
}

impl MimeType {
    /// Infers a resource's MIME type according to the extension of its path
    pub fn from_extension(resource_path: &str) -> Self {
        if let Some(extension_index) = resource_path.rfind('.') {
            match &resource_path[extension_index + 1..] {
                "gif" => MimeType::ImageGif,
                "html" => MimeType::TextHtml,
                "js" => MimeType::ApplicationJavascript,
                "mp3" => MimeType::AudioMp3,
                "mp4" => MimeType::VideoMp4,
                "png" => MimeType::ImagePng,
                "txt" => MimeType::TextPlain,
                _ => {
                    eprintln!("Unrecognized file extension on: {:?}", resource_path);
                    MimeType::Unknown
                }
            }
        } else {
            return MimeType::Unknown
        }
    }
}

impl From<&str> for MimeType {
    fn from(v: &str) -> Self {
        match v {
            "image/gif" => MimeType::ImageGif,
            "text/html" => MimeType::TextHtml,
            "application/javascript" => MimeType::ApplicationJavascript,
            "audio/mp3" => MimeType::AudioMp3,
            "video/mp4" => MimeType::VideoMp4,
            "image/png" => MimeType::ImagePng,
            "text/plain" => MimeType::TextPlain,
            _ => MimeType::Unknown,
        }
    }
}

impl From<MimeType> for String {
    fn from(v: MimeType) -> Self {
        match v {
            MimeType::ImageGif => "image/gif",
            MimeType::TextHtml => "text/html",
            MimeType::ApplicationJavascript => "application/javascript",
            MimeType::AudioMp3 => "audio/mp3",
            MimeType::VideoMp4 => "video/mp4",
            MimeType::ImagePng => "image/png",
            MimeType::TextPlain => "text/plain",
            MimeType::Unknown => "application/octet-stream",
        }.to_owned()
    }
}

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

const REDIRECTABLE_RESOURCES_DECLARATION: &'static str = "const redirectableResources = new Map([";

/// Reads data from a a file in the format of uBlock Origin's `redirect-engine.js` file to
/// determine the files in the `web_accessible_resources` directory, as well as any of their
/// aliases.
///
/// This is read from the `redirectableResources` map.
fn read_redirectable_resource_mapping(mapfile_data: &str) -> Vec<ResourceProperties> {
    let mut resource_properties = Vec::new();

    let mut current_resource: Option<ResourceProperties> = None;
    let mut found_mapping = false;

    for line in mapfile_data.lines().skip_while(|line| *line != REDIRECTABLE_RESOURCES_DECLARATION).skip(1) {
        found_mapping = true;

        let key_capture = MAP_NAME_KEY_RE.captures(line);
        if let Some(key_capture) = key_capture {
            // unwrap is safe because first capture group must be populated
            let key = key_capture.get(1).unwrap().as_str();
            current_resource = Some(ResourceProperties { name: key.to_owned(), alias: None, data: None});
            continue;
        }

        let prop_captures = MAP_PROPERTY_KEY_RE.captures(line);
        if let Some(prop_captures) = prop_captures {
            // unwraps are safe because both capture groups must be populated
            let property_key = prop_captures.get(1).unwrap().as_str();
            let property_value = prop_captures.get(2).unwrap().as_str();

            if let Some(ref mut current_resource) = current_resource {
                match property_key {
                    "alias" => current_resource.alias = Some(property_value.to_owned()),
                    "data" => current_resource.data = Some(property_value.to_owned()),
                    other => panic!("unexpected key: {}", other),
                }
            } else {
                panic!("could not find beginning of resource in alias map");
            }
            continue;
        }

        if MAP_END_KEY_RE.is_match(line) {
            resource_properties.push(current_resource.take().expect("closed a resource entry before opening a new one"));
            continue;
        }

        if MAP_END_RE.is_match(line) {
            if let Some(current_resource) = current_resource {
                panic!("encountered end of alias map while {} was unclosed", current_resource.name);
            }
            break;
        }

        panic!("encountered unexpected line {:?}", line);
    }

    if !found_mapping {
        panic!("failed to find mapping");
    }

    resource_properties
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
    fn test_resource_assembly() {
        let web_accessible_resource_dir = Path::new("data/test/fake-uBO-files/web_accessible_resources");
        let redirect_engine_path = Path::new("data/test/fake-uBO-files/redirect-engine.js");
        let scriptlets_path = Path::new("data/test/fake-uBO-files/scriptlets.js");
        let mut resources = assemble_web_accessible_resources(web_accessible_resource_dir, redirect_engine_path);
        resources.append(&mut assemble_scriptlet_resources(scriptlets_path));

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

        let expected_template_resources = vec![
            "abort-current-inline-script.js",
            "abort-on-property-read.js",
            "abort-on-property-write.js",
            "addEventListener-defuser.js",
            "addEventListener-logger.js",
            "nano-setInterval-booster.js",
            "nano-setTimeout-booster.js",
            "noeval-if.js",
            "remove-attr.js",
            "set-constant.js",
            "setInterval-defuser.js",
            "setTimeout-defuser.js",
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
                    if let ResourceType::Mime(_) = resource.kind {
                        resource.name == name
                    } else {
                        false
                    }
                }).is_some());
        }

        for name in expected_template_resources {
            assert!(resources.iter()
                .find(|resource| {
                    if let ResourceType::Template = resource.kind {
                        resource.name == name
                    } else {
                        false
                    }
                })
                .is_some(), "failed to find {}", name);
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
}
