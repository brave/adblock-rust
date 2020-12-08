//! In adblocking terms, resources are special placeholder scripts, images,
//! video files, etc. that can be returned as drop-in replacements for harmful
//! equivalents from remote servers. Resources also encompass scriptlets, which
//! can be injected into pages to inhibit malicious behavior.

#[cfg(feature = "resource-assembler")]
pub mod resource_assembler;

mod scriptlet_resource_storage;
pub(crate) use scriptlet_resource_storage::ScriptletResourceStorage;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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
#[serde(from = "std::borrow::Cow<'static, str>")]
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RedirectResource {
    pub content_type: String,
    pub data: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct RedirectResourceStorage {
    pub resources: HashMap<String, RedirectResource>,
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
            MimeType::Unknown
        }
    }
}

impl RedirectResourceStorage {
    pub fn from_resources(resources: &[Resource]) -> Self {
        let mut redirectable_resources: HashMap<String, RedirectResource> = HashMap::new();

        resources.iter().filter_map(|descriptor| {
            if let ResourceType::Mime(ref content_type) = descriptor.kind {
                let resource = RedirectResource {
                    content_type: content_type.clone().into(),
                    data: descriptor.content.to_owned(),
                };
                Some((descriptor.name.to_owned(), descriptor.aliases.to_owned(), resource))
            } else {
                None
            }
        }).for_each(|(name, res_aliases, resource)| {
            res_aliases.iter().for_each(|alias| {
                redirectable_resources.insert(alias.to_owned(), resource.clone());
            });
            redirectable_resources.insert(name, resource);
        });

        Self {
            resources: redirectable_resources,
        }
    }

    pub fn get_resource(&self, name: &str) -> Option<&RedirectResource> {
        self.resources.get(name)
    }

    pub fn add_resource(&mut self, resource: &Resource) {
        if let ResourceType::Mime(ref content_type) = resource.kind {
            let name = resource.name.to_owned();
            let redirect_resource = RedirectResource {
                content_type: content_type.clone().into(),
                data: resource.content.to_owned(),
            };
            resource.aliases.iter().for_each(|alias| {
                self.resources.insert(alias.to_owned(), redirect_resource.clone());
            });
            self.resources.insert(name, redirect_resource);
        }
    }
}

impl From<std::borrow::Cow<'static, str>> for MimeType {
    fn from(v: std::borrow::Cow<'static, str>) -> Self {
        match v.as_ref() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_resource_by_name() {
        let mut storage = RedirectResourceStorage::default();
        storage.add_resource(&Resource {
            name: "name.js".to_owned(),
            aliases: vec![],
            kind: ResourceType::Mime(MimeType::ApplicationJavascript),
            content: base64::encode("resource data"),
        });

        assert_eq!(storage.get_resource("name.js"), Some(&RedirectResource {
            content_type: "application/javascript".to_owned(),
            data: base64::encode("resource data"),
        }));
    }

    #[test]
    fn get_resource_by_alias() {
        let mut storage = RedirectResourceStorage::default();
        storage.add_resource(&Resource {
            name: "name.js".to_owned(),
            aliases: vec!["alias.js".to_owned()],
            kind: ResourceType::Mime(MimeType::ApplicationJavascript),
            content: base64::encode("resource data"),
        });

        assert_eq!(storage.get_resource("alias.js"), Some(&RedirectResource {
            content_type: "application/javascript".to_owned(),
            data: base64::encode("resource data"),
        }));
    }
}

