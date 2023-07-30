//! In adblocking terms, [`Resource`]s are special placeholder scripts, images,
//! video files, etc. that can be returned as drop-in replacements for harmful
//! equivalents from remote servers. Resources also encompass scriptlets, which
//! can be injected into pages to inhibit malicious behavior.
//!
//! If the `resource-assembler` feature is enabled, the
#![cfg_attr(not(feature = "resource-assembler"), doc="`resource_assembler`")]
#![cfg_attr(feature = "resource-assembler", doc="[`resource_assembler`]")]
//! module will assist with the construction of [`Resource`]s directly from the uBlock Origin
//! project.

#[cfg(feature = "resource-assembler")]
pub mod resource_assembler;

mod resource_storage;
#[doc(inline)]
pub use resource_storage::{AddResourceError, ResourceStorage, ScriptletResourceError};

use memchr::memrchr as find_char_reverse;
use serde::{Deserialize, Serialize};

/// Struct representing a resource that can be used by an adblocking engine.
#[derive(Serialize, Deserialize, Clone)]
pub struct Resource {
    /// Represents the primary name of the resource, often a filename
    pub name: String,
    /// Represents secondary names that can be used to access the resource
    pub aliases: Vec<String>,
    /// How to interpret the resource data within `content`
    pub kind: ResourceType,
    /// The resource data, encoded using standard base64 configuration
    pub content: String,
}

/// Different ways that the data within the `content` field of a `Resource` can be interpreted.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    /// Interpret the data according to the MIME type represented by `type`
    Mime(MimeType),
    /// Interpret the data as a Javascript scriptlet template, with embedded template
    /// parameters in the form of `{{1}}`, `{{2}}`, etc. Note that `Mime(ApplicationJavascript)`
    /// can still be used as a templated resource, for compatibility purposes.
    Template,
}

/// Acceptable MIME types for resources used by `$redirect` and `+js(...)` adblock rules.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(into = "&str")]
#[serde(from = "std::borrow::Cow<'static, str>")]
pub enum MimeType {
    TextCss,
    ImageGif,
    TextHtml,
    ApplicationJavascript,
    AudioMp3,
    VideoMp4,
    ImagePng,
    TextPlain,
    TextXml,
    Unknown,
}

impl MimeType {
    /// Infers a resource's MIME type according to the extension of its path
    pub fn from_extension(resource_path: &str) -> Self {
        if let Some(extension_index) = find_char_reverse(b'.', resource_path.as_bytes()) {
            match &resource_path[extension_index + 1..] {
                "css" => MimeType::TextCss,
                "gif" => MimeType::ImageGif,
                "html" => MimeType::TextHtml,
                "js" => MimeType::ApplicationJavascript,
                "mp3" => MimeType::AudioMp3,
                "mp4" => MimeType::VideoMp4,
                "png" => MimeType::ImagePng,
                "txt" => MimeType::TextPlain,
                "xml" => MimeType::TextXml,
                _ => {
                    #[cfg(test)]
                    eprintln!("Unrecognized file extension on: {:?}", resource_path);
                    MimeType::Unknown
                }
            }
        } else {
            MimeType::Unknown
        }
    }

    /// Should the MIME type decode as valid UTF8?
    pub fn is_textual(&self) -> bool {
        matches!(self, MimeType::ApplicationJavascript | MimeType::TextCss | MimeType::TextPlain | MimeType::TextHtml | MimeType::TextXml)
    }
}

impl From<&str> for MimeType {
    fn from(v: &str) -> Self {
        match v {
            "text/css" => MimeType::TextCss,
            "image/gif" => MimeType::ImageGif,
            "text/html" => MimeType::TextHtml,
            "application/javascript" => MimeType::ApplicationJavascript,
            "audio/mp3" => MimeType::AudioMp3,
            "video/mp4" => MimeType::VideoMp4,
            "image/png" => MimeType::ImagePng,
            "text/plain" => MimeType::TextPlain,
            "text/xml" => MimeType::TextXml,
            _ => MimeType::Unknown,
        }
    }
}

impl From<&MimeType> for &str {
    fn from(v: &MimeType) -> Self {
        match v {
            MimeType::TextCss => "text/css",
            MimeType::ImageGif => "image/gif",
            MimeType::TextHtml => "text/html",
            MimeType::ApplicationJavascript => "application/javascript",
            MimeType::AudioMp3 => "audio/mp3",
            MimeType::VideoMp4 => "video/mp4",
            MimeType::ImagePng => "image/png",
            MimeType::TextPlain => "text/plain",
            MimeType::TextXml => "text/xml",
            MimeType::Unknown => "application/octet-stream",
        }
    }
}

// Required for `#[serde(from = "std::borrow::Cow<'static, str>")]`
impl From<std::borrow::Cow<'static, str>> for MimeType {
    fn from(v: std::borrow::Cow<'static, str>) -> Self {
        v.as_ref().into()
    }
}

// Required for `#[serde(into = &str)]`
impl From<MimeType> for &str {
    fn from(v: MimeType) -> Self {
        (&v).into()
    }
}

impl std::fmt::Display for MimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}
