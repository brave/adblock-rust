use serde::{Serialize, Deserialize};

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
