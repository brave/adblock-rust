use crate::resources::PermissionMask;
use memchr::memchr as find_char;

/// Returns the first token of a CSS selector.
///
/// This should only be called once `selector` has been verified to start with either a "#" or "."
/// character.
pub(crate) fn key_from_selector(selector: &str) -> Option<String> {
  use once_cell::sync::Lazy;
  use regex::Regex;

  static RE_PLAIN_SELECTOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[#.][\w\\-]+").unwrap());
  static RE_PLAIN_SELECTOR_ESCAPED: Lazy<Regex> =
      Lazy::new(|| Regex::new(r"^[#.](?:\\[0-9A-Fa-f]+ |\\.|\w|-)+").unwrap());
  static RE_ESCAPE_SEQUENCE: Lazy<Regex> =
      Lazy::new(|| Regex::new(r"\\([0-9A-Fa-f]+ |.)").unwrap());

  // If there are no escape characters in the selector, just take the first class or id token.
  let mat = RE_PLAIN_SELECTOR.find(selector);
  if let Some(location) = mat {
      let key = &location.as_str();
      if find_char(b'\\', key.as_bytes()).is_none() {
          return Some((*key).into());
      }
  } else {
      return None;
  }

  // Otherwise, the characters in the selector must be escaped.
  let mat = RE_PLAIN_SELECTOR_ESCAPED.find(selector);
  if let Some(location) = mat {
      let mut key = String::with_capacity(selector.len());
      let escaped = &location.as_str();
      let mut beginning = 0;
      let mat = RE_ESCAPE_SEQUENCE.captures_iter(escaped);
      for capture in mat {
          // Unwrap is safe because the 0th capture group is the match itself
          let location = capture.get(0).unwrap();
          key += &escaped[beginning..location.start()];
          beginning = location.end();
          // Unwrap is safe because there is a capture group specified in the regex
          let capture = capture.get(1).unwrap().as_str();
          if capture.chars().count() == 1 {
              // Check number of unicode characters rather than byte length
              key += capture;
          } else {
              // This u32 conversion can overflow
              let codepoint = u32::from_str_radix(&capture[..capture.len() - 1], 16).ok()?;

              // Not all u32s are valid Unicode codepoints
              key += &core::char::from_u32(codepoint)?.to_string();
          }
      }
      Some(key + &escaped[beginning..])
  } else {
      None
  }
}


/// Exists to use common logic for binning filters correctly
#[derive(Clone)]
pub(crate) enum SpecificFilterType {
    Hide(String),
    Unhide(String),
    InjectScript((String, PermissionMask)),
    UninjectScript((String, PermissionMask)),
    ProceduralOrAction(String),
    ProceduralOrActionException(String),
}

impl SpecificFilterType {
    pub(crate) fn negated(self) -> Self {
        match self {
            Self::Hide(s) => Self::Unhide(s),
            Self::Unhide(s) => Self::Hide(s),
            Self::InjectScript(s) => Self::UninjectScript(s),
            Self::UninjectScript(s) => Self::InjectScript(s),
            Self::ProceduralOrAction(s) => Self::ProceduralOrActionException(s),
            Self::ProceduralOrActionException(s) => Self::ProceduralOrAction(s),
        }
    }
}

/// Encodes permission bits in the last byte of a script string
/// Returns the script with permission byte prepended
pub(crate) fn encode_script_with_permission(mut script: String, permission: PermissionMask) -> String {
  script.push(permission.to_bits() as char);
  script
}

/// Decodes permission bits from the last byte of a script string
/// Returns (permission, script) tuple
pub(crate) fn decode_script_with_permission(encoded_script: &str) -> (PermissionMask, &str) {
  if encoded_script.is_empty() {
      return (PermissionMask::default(), encoded_script);
  }

  let last_char = encoded_script.chars().last().unwrap();
  let permission_bits = last_char as u8;
  let permission = PermissionMask::from_bits(permission_bits);
  let script = &encoded_script[..encoded_script.len() - 1];
  (permission, script)
}
