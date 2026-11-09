//! Parsing of `$replace=/pattern/replacement/flags` options.

use serde::Serialize;
use thiserror::Error;

use super::abstract_network::is_escaped_delimiter;

/// A `$replace` option split into the values consumed by the caller.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReplaceRule {
    /// `$replace=/pattern/replacement/flags`
    Regex {
        /// JavaScript regular-expression source, without the surrounding `/` delimiters.
        pattern: String,
        /// JavaScript replacement string.
        replacement: String,
        /// JavaScript regular-expression flags.
        flags: String,
    },
    /// `$replace=json:<query>`: a JSONPath query applied to the whole response body.
    Json { query: String },
    /// `$replace=jsonl:<query>`: a JSONPath query applied to each line of the response body.
    JsonLines { query: String },
}

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[error("invalid $replace option syntax")]
pub struct ReplaceOptionParseError;

/// Removes the backslash from every `\x` where `is_escapable(x)`.
fn unescape(value: &str, is_escapable: impl Fn(char) -> bool) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek().is_some_and(|&next| is_escapable(next)) {
            result.push(chars.next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

impl ReplaceRule {
    /// Splits a raw `$replace` option without compiling or validating its regular expression or
    /// query.
    pub(crate) fn parse(value: &str) -> Result<Self, ReplaceOptionParseError> {
        // Anything that does not start with a `/` is a type name up to the first colon.
        let json_query = |query: &str| {
            if query.is_empty() {
                Err(ReplaceOptionParseError)
            } else {
                Ok(unescape(query, |ch| ch == ','))
            }
        };
        if let Some(query) = value.strip_prefix("json:") {
            return Ok(Self::Json {
                query: json_query(query)?,
            });
        }
        if let Some(query) = value.strip_prefix("jsonl:") {
            return Ok(Self::JsonLines {
                query: json_query(query)?,
            });
        }

        if !value.starts_with('/') {
            return Err(ReplaceOptionParseError);
        }

        let bytes = value.as_bytes();
        let is_delimiter =
            |&index: &usize| bytes[index] == b'/' && !is_escaped_delimiter(bytes, index);

        // The pattern ends at the first unescaped `/`. The replacement ends at the last one, so
        // that lists may leave `/` unescaped inside the replacement (e.g. `</div>`).
        let pattern_end = (1..bytes.len())
            .find(is_delimiter)
            .ok_or(ReplaceOptionParseError)?;
        // An empty pattern would match everywhere.
        if pattern_end == 1 {
            return Err(ReplaceOptionParseError);
        }
        let replacement_end = (pattern_end + 1..bytes.len())
            .rev()
            .find(is_delimiter)
            .ok_or(ReplaceOptionParseError)?;

        let flags = &value[replacement_end + 1..];
        if !flags
            .bytes()
            .all(|flag| matches!(flag, b'i' | b'm' | b's' | b'g' | b'u'))
        {
            return Err(ReplaceOptionParseError);
        }

        Ok(Self::Regex {
            // `\$` is a literal dollar in a regex, so only the list syntax is unescaped there.
            pattern: unescape(&value[1..pattern_end], |ch| matches!(ch, ',' | '/')),
            replacement: unescape(&value[pattern_end + 1..replacement_end], |ch| {
                matches!(ch, ',' | '$' | '/')
            }),
            flags: flags.to_string(),
        })
    }
}
#[cfg(test)]
#[path = "../../tests/unit/filters/replace.rs"]
mod unit_tests;
