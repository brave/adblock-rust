// Copyright 2013-2016 The rust-url developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[allow(unused_imports, deprecated)]
use std::ascii::AsciiExt;

use std::error::Error;
use std::fmt::{self, Formatter, Write};
use std::str;

use url::Host;
use url::idna;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::percent_encoding::{utf8_percent_encode, USERINFO_ENCODE_SET};
use std::ops::{Range, RangeFrom, RangeTo};

#[derive(Clone)]
pub struct Hostname {
    serialization: String,

    // Components
    scheme_end: u32,  // Before ':'
    username_end: u32,  // Before ':' (if a password is given) or '@' (if not)
    host_start: u32,
    host_end: u32,
    host: HostInternal
}

impl Hostname {
    #[inline]
    pub fn parse(input: &str) -> Result<Hostname, ParseError> {
        Parser {
            serialization: String::with_capacity(input.len()),
            violation_fn: ViolationFn::NoOp,
        }.parse_url(input)
    }

    /// Equivalent to `url.host().is_some()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert!(url.has_host());
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert!(!url.has_host());
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert!(!url.has_host());
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn has_host(&self) -> bool {
        !matches!(self.host, HostInternal::None)
    }

    /// Return the string representation of the host (domain or IP address) for this URL, if any.
    ///
    /// Non-ASCII domains are punycode-encoded per IDNA.
    /// IPv6 addresses are given between `[` and `]` brackets.
    ///
    /// Cannot-be-a-base URLs (typical of `data:` and `mailto:`) and some `file:` URLs
    /// don’t have a host.
    ///
    /// See also the `host` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use url::Url;
    /// # use url::ParseError;
    ///
    /// # fn run() -> Result<(), ParseError> {
    /// let url = Url::parse("https://127.0.0.1/index.html")?;
    /// assert_eq!(url.host_str(), Some("127.0.0.1"));
    ///
    /// let url = Url::parse("ftp://rms@example.com")?;
    /// assert_eq!(url.host_str(), Some("example.com"));
    ///
    /// let url = Url::parse("unix:/run/foo.socket")?;
    /// assert_eq!(url.host_str(), None);
    ///
    /// let url = Url::parse("data:text/plain,Stuff")?;
    /// assert_eq!(url.host_str(), None);
    /// # Ok(())
    /// # }
    /// # run().unwrap();
    /// ```
    pub fn host_str(&self) -> Option<&str> {
        if self.has_host() {
            Some(self.slice(self.host_start..self.host_end))
        } else {
            None
        }
    }

    // Private helper methods:

    #[inline]
    fn slice<R>(&self, range: R) -> &str where R: RangeArg {
        range.slice_of(&self.serialization)
    }
}

trait RangeArg {
    fn slice_of<'a>(&self, s: &'a str) -> &'a str;
}

impl RangeArg for Range<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize .. self.end as usize]
    }
}

impl RangeArg for RangeFrom<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[self.start as usize ..]
    }
}

impl RangeArg for RangeTo<u32> {
    #[inline]
    fn slice_of<'a>(&self, s: &'a str) -> &'a str {
        &s[.. self.end as usize]
    }
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum HostInternal {
    None,
    Domain,
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
}

impl<S> From<Host<S>> for HostInternal {
    fn from(host: Host<S>) -> HostInternal {
        match host {
            Host::Domain(_) => HostInternal::Domain,
            Host::Ipv4(address) => HostInternal::Ipv4(address),
            Host::Ipv6(address) => HostInternal::Ipv6(address),
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

macro_rules! simple_enum_error {
    ($($name: ident => $description: expr,)+) => {
        /// Errors that can occur during parsing.
        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum ParseError {
            $(
                $name,
            )+
        }

        impl Error for ParseError {
            fn description(&self) -> &str {
                match *self {
                    $(
                        ParseError::$name => $description,
                    )+
                }
            }
        }
    }
}

simple_enum_error! {
    EmptyHost => "empty host",
    IdnaError => "invalid international domain name",
    InvalidPort => "invalid port number",
    InvalidIpv4Address => "invalid IPv4 address",
    InvalidIpv6Address => "invalid IPv6 address",
    InvalidDomainCharacter => "invalid domain character",
    HostParseError => "internal host parse error",
    RelativeUrlWithoutBase => "relative URL without a base",
    RelativeUrlWithCannotBeABaseBase => "relative URL with a cannot-be-a-base base",
    SetHostOnCannotBeABaseUrl => "a cannot-be-a-base URL doesn’t have a host to set",
    Overflow => "URLs more than 4 GB are not supported",
    FileUrlNotSupported => "file URLs are not supported",
}

#[cfg(feature = "heapsize")]
known_heap_size!(0, ParseError);

impl fmt::Display for ParseError {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.description().fmt(fmt)
    }
}

impl From<idna::uts46::Errors> for ParseError {
    fn from(_: idna::uts46::Errors) -> ParseError { ParseError::IdnaError }
}

macro_rules! syntax_violation_enum {
    ($($name: ident => $description: expr,)+) => {
        /// Non-fatal syntax violations that can occur during parsing.
        #[derive(PartialEq, Eq, Clone, Copy, Debug)]
        pub enum SyntaxViolation {
            $(
                $name,
            )+
        }

        impl SyntaxViolation {
            pub fn description(&self) -> &'static str {
                match *self {
                    $(
                        SyntaxViolation::$name => $description,
                    )+
                }
            }
        }
    }
}

syntax_violation_enum! {
    Backslash => "backslash",
    C0SpaceIgnored =>
        "leading or trailing control or space character are ignored in URLs",
    EmbeddedCredentials =>
        "embedding authentication information (username or password) \
         in an URL is not recommended",
    ExpectedDoubleSlash => "expected //",
    ExpectedFileDoubleSlash => "expected // after file:",
    FileWithHostAndWindowsDrive => "file: with host and Windows drive letter",
    NonUrlCodePoint => "non-URL code point",
    NullInFragment => "NULL characters are ignored in URL fragment identifiers",
    PercentDecode => "expected 2 hex digits after %",
    TabOrNewlineIgnored => "tabs or newlines are ignored in URLs",
    UnencodedAtSign => "unencoded @ sign in username or password",
}

#[cfg(feature = "heapsize")]
known_heap_size!(0, SyntaxViolation);

impl fmt::Display for SyntaxViolation {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        self.description().fmt(fmt)
    }
}

#[derive(Copy, Clone)]
pub enum SchemeType {
    File,
    SpecialNotFile,
    NotSpecial,
}

impl SchemeType {
    pub fn is_special(&self) -> bool {
        !matches!(*self, SchemeType::NotSpecial)
    }

    pub fn is_file(&self) -> bool {
        matches!(*self, SchemeType::File)
    }

    pub fn from(s: &str) -> Self {
        match s {
            "http" | "https" | "ws" | "wss" | "ftp" | "gopher" => SchemeType::SpecialNotFile,
            "file" => SchemeType::File,
            _ => SchemeType::NotSpecial,
        }
    }
}

#[derive(Clone)]
pub struct Input<'i> {
    chars: str::Chars<'i>,
}

impl<'i> Input<'i> {
    pub fn new(input: &'i str) -> Self {
        Input::with_log(input, ViolationFn::NoOp)
    }

    pub fn with_log(original_input: &'i str, vfn: ViolationFn) -> Self {
        let input = original_input.trim_matches(c0_control_or_space);
        if vfn.is_set() {
            if input.len() < original_input.len() {
                vfn.call(SyntaxViolation::C0SpaceIgnored)
            }
            if input.chars().any(|c| matches!(c, '\t' | '\n' | '\r')) {
                vfn.call(SyntaxViolation::TabOrNewlineIgnored)
            }
        }
        Input { chars: input.chars() }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clone().next().is_none()
    }

    #[inline]
    fn starts_with<P: Pattern>(&self, p: P) -> bool {
        p.split_prefix(&mut self.clone())
    }

    #[inline]
    pub fn split_prefix<P: Pattern>(&self, p: P) -> Option<Self> {
        let mut remaining = self.clone();
        if p.split_prefix(&mut remaining) {
            Some(remaining)
        } else {
            None
        }
    }

    #[inline]
    fn split_first(&self) -> (Option<char>, Self) {
        let mut remaining = self.clone();
        (remaining.next(), remaining)
    }

    #[inline]
    fn count_matching<F: Fn(char) -> bool>(&self, f: F) -> (u32, Self) {
        let mut count = 0;
        let mut remaining = self.clone();
        loop {
            let mut input = remaining.clone();
            if matches!(input.next(), Some(c) if f(c)) {
                remaining = input;
                count += 1;
            } else {
                return (count, remaining)
            }
        }
    }

    #[inline]
    fn next_utf8(&mut self) -> Option<(char, &'i str)> {
        loop {
            let utf8 = self.chars.as_str();
            match self.chars.next() {
                Some(c) => {
                    if !matches!(c, '\t' | '\n' | '\r') {
                        return Some((c, &utf8[..c.len_utf8()]))
                    }
                }
                None => return None
            }
        }
    }
}

pub trait Pattern {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool;
}

impl Pattern for char {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool { input.next() == Some(self) }
}

impl<'a> Pattern for &'a str {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool {
        for c in self.chars() {
            if input.next() != Some(c) {
                return false
            }
        }
        true
    }
}

impl<F: FnMut(char) -> bool> Pattern for F {
    fn split_prefix<'i>(self, input: &mut Input<'i>) -> bool { input.next().map_or(false, self) }
}

impl<'i> Iterator for Input<'i> {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        self.chars.by_ref().find(|&c| !matches!(c, '\t' | '\n' | '\r'))
    }
}

/// Wrapper for syntax violation callback functions.
#[derive(Copy, Clone)]
pub enum ViolationFn<'a> {
    NewFn(&'a (Fn(SyntaxViolation) + 'a)),
    OldFn(&'a (Fn(&'static str) + 'a)),
    NoOp
}

impl<'a> ViolationFn<'a> {
    /// Call with a violation.
    pub fn call(self, v: SyntaxViolation) {
        match self {
            ViolationFn::NewFn(f) => f(v),
            ViolationFn::OldFn(f) => f(v.description()),
            ViolationFn::NoOp => {}
        }
    }

    /// Call with a violation, if provided test returns true. Avoids
    /// the test entirely if `NoOp`.
    pub fn call_if<F>(self, v: SyntaxViolation, test: F)
        where F: Fn() -> bool
    {
        match self {
            ViolationFn::NewFn(f) => if test() { f(v) },
            ViolationFn::OldFn(f) => if test() { f(v.description()) },
            ViolationFn::NoOp => {} // avoid test
        }
    }

    /// True if not `NoOp`
    pub fn is_set(self) -> bool {
        match self {
            ViolationFn::NoOp => false,
            _ => true
        }
    }
}

impl<'a> fmt::Debug for ViolationFn<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ViolationFn::NewFn(_) => write!(f, "NewFn(Fn(SyntaxViolation))"),
            ViolationFn::OldFn(_) => write!(f, "OldFn(Fn(&'static str))"),
            ViolationFn::NoOp     => write!(f, "NoOp")
        }
    }
}

pub struct Parser<'a> {
    pub serialization: String,
    pub violation_fn: ViolationFn<'a>,
}

impl<'a> Parser<'a> {

    /// https://url.spec.whatwg.org/#concept-basic-url-parser
    pub fn parse_url(mut self, input: &str) -> ParseResult<Hostname> {
        // println!("Parse {}", input);
        let input = Input::with_log(input, self.violation_fn);
        if let Ok(remaining) = self.parse_scheme(input.clone()) {
            return self.parse_with_scheme(remaining)
        }

        // No-scheme state
        Err(ParseError::RelativeUrlWithoutBase)
    }

    pub fn parse_scheme<'i>(&mut self, mut input: Input<'i>) -> Result<Input<'i>, ()> {
        if input.is_empty() || !input.starts_with(ascii_alpha) {
            return Err(())
        }
        debug_assert!(self.serialization.is_empty());
        while let Some(c) = input.next() {
            match c {
                'a'...'z' | 'A'...'Z' | '0'...'9' | '+' | '-' | '.' => {
                    self.serialization.push(c.to_ascii_lowercase())
                }
                ':' => return Ok(input),
                _ => {
                    self.serialization.clear();
                    return Err(())
                }
            }
        }
        
        Err(())
    }

    fn parse_with_scheme(mut self, input: Input) -> ParseResult<Hostname> {
        use SyntaxViolation::{ExpectedFileDoubleSlash, ExpectedDoubleSlash};
        let scheme_end = to_u32(self.serialization.len())?;
        let scheme_type = SchemeType::from(&self.serialization);
        self.serialization.push(':');
        match scheme_type {
            SchemeType::File => {
                // println!("Parse file - not supported");
                Err(ParseError::FileUrlNotSupported)
            }
            SchemeType::SpecialNotFile => {
                // println!("Parse special, not file");
                // special relative or authority state
                let (slashes_count, remaining) = input.count_matching(|c| matches!(c, '/' | '\\'));
                // special authority slashes state
                self.violation_fn.call_if(ExpectedDoubleSlash, || {
                    input.clone().take_while(|&c| matches!(c, '/' | '\\'))
                    .collect::<String>() != "//"
                });
                // println!("Parse after double slash {}", remaining.chars.as_str());
                self.after_double_slash(remaining, scheme_type, scheme_end)
            }
            SchemeType::NotSpecial => {
                // println!("Parse non special {}", &self.serialization);
                self.parse_non_special(input, scheme_type, scheme_end)
            }
        }
    }

    /// Scheme other than file, http, https, ws, ws, ftp, gopher.
    fn parse_non_special(mut self, input: Input, scheme_type: SchemeType, scheme_end: u32)
                         -> ParseResult<Hostname> {
        // path or authority state (
        if let Some(input) = input.split_prefix("//") {
            return self.after_double_slash(input, scheme_type, scheme_end)
        }
        // Anarchist URL (no authority)
        let path_start = to_u32(self.serialization.len())?;
        let username_end = path_start;
        let host_start = path_start;
        let host_end = path_start;
        let host = HostInternal::None;

        Ok(Hostname {
            serialization: self.serialization,
            scheme_end: scheme_end,
            username_end: username_end,
            host_start: host_start,
            host_end: host_end,
            host: host.into()
        })
    }

    fn after_double_slash(mut self, input: Input, scheme_type: SchemeType, scheme_end: u32)
                          -> ParseResult<Hostname> {
        self.serialization.push('/');
        self.serialization.push('/');
        // authority state
        let (username_end, remaining) = self.parse_userinfo(input, scheme_type)?;
        // host state
        let host_start = to_u32(self.serialization.len())?;
        // println!("Parse host {}", remaining.chars.as_str());
        let (host, _) = Parser::parse_host(remaining, scheme_type)?;
        write!(&mut self.serialization, "{}", host).unwrap();
        let host_end = to_u32(self.serialization.len())?;
        // println!("Return hostname {} from {} to {}", self.serialization, host_start, host_end);
        Ok(Hostname {
            serialization: self.serialization,
            scheme_end: scheme_end,
            username_end: username_end,
            host_start: host_start,
            host_end: host_end,
            host: host.into()
        })
    }

    /// Return (username_end, remaining)
    fn parse_userinfo<'i>(&mut self, mut input: Input<'i>, scheme_type: SchemeType)
                          -> ParseResult<(u32, Input<'i>)> {
        let mut last_at = None;
        let mut remaining = input.clone();
        let mut char_count = 0;
        while let Some(c) = remaining.next() {
            match c {
                '@' => {
                    if last_at.is_some() {
                        self.violation_fn.call(SyntaxViolation::UnencodedAtSign)
                    } else {
                        self.violation_fn.call(SyntaxViolation::EmbeddedCredentials)
                    }
                    last_at = Some((char_count, remaining.clone()))
                },
                '/' | '?' | '#' => break,
                '\\' if scheme_type.is_special() => break,
                _ => (),
            }
            char_count += 1;
        }
        let (mut userinfo_char_count, remaining) = match last_at {
            None => return Ok((to_u32(self.serialization.len())?, input)),
            Some((0, remaining)) => return Ok((to_u32(self.serialization.len())?, remaining)),
            Some(x) => x
        };

        let mut username_end = None;
        let mut has_password = false;
        let mut has_username = false;
        while userinfo_char_count > 0 {
            let (c, utf8_c) = input.next_utf8().unwrap();
            userinfo_char_count -= 1;
            if c == ':' && username_end.is_none() {
                // Start parsing password
                username_end = Some(to_u32(self.serialization.len())?);
                // We don't add a colon if the password is empty
                if userinfo_char_count > 0 {
                    self.serialization.push(':');
                    has_password = true;
                }
            } else {
                if !has_password {
                    has_username = true;
                }
                self.check_url_code_point(c, &input);
                self.serialization.extend(utf8_percent_encode(utf8_c, USERINFO_ENCODE_SET));
            }
        }
        let username_end = match username_end {
            Some(i) => i,
            None => to_u32(self.serialization.len())?,
        };
        if has_username || has_password {
            self.serialization.push('@');
        }
        Ok((username_end, remaining))
    }

    pub fn parse_host(mut input: Input, scheme_type: SchemeType)
                             -> ParseResult<(Host<String>, Input)> {
        // Undo the Input abstraction here to avoid allocating in the common case
        // where the host part of the input does not contain any tab or newline
        let input_str = input.chars.as_str();
        let mut inside_square_brackets = false;
        let mut has_ignored_chars = false;
        let mut non_ignored_chars = 0;
        let mut bytes = 0;
        for c in input_str.chars() {
            match c {
                ':' if !inside_square_brackets => break,
                '\\' if scheme_type.is_special() => break,
                '/' | '?' | '#' => break,
                '\t' | '\n' | '\r' => {
                    has_ignored_chars = true;
                }
                '[' => {
                    inside_square_brackets = true;
                    non_ignored_chars += 1
                }
                ']' => {
                    inside_square_brackets = false;
                    non_ignored_chars += 1
                }
                _ => non_ignored_chars += 1
            }
            bytes += c.len_utf8();
        }
        let replaced: String;
        let host_str;
        {
            let host_input = input.by_ref().take(non_ignored_chars);
            if has_ignored_chars {
                replaced = host_input.collect();
                host_str = &*replaced
            } else {
                for _ in host_input {}
                host_str = &input_str[..bytes]
            }
        }
        if scheme_type.is_special() && host_str.is_empty() {
            return Err(ParseError::EmptyHost)
        }
        if !scheme_type.is_special() {
            match Host::parse_opaque(host_str) {
                Ok(host) => return Ok((host, input)),
                Err(_) => return Err(ParseError::HostParseError)
            }
        }
        match Host::parse(host_str) {
            Ok(host) => return Ok((host, input)),
            Err(_) => return Err(ParseError::HostParseError)
        }
    }

    
    fn check_url_code_point(&self, c: char, input: &Input) {
        let vfn = self.violation_fn;
        if vfn.is_set() {
            if c == '%' {
                let mut input = input.clone();
                if !matches!((input.next(), input.next()), (Some(a), Some(b))
                             if is_ascii_hex_digit(a) && is_ascii_hex_digit(b)) {
                    vfn.call(SyntaxViolation::PercentDecode)
                }
            } else if !is_url_code_point(c) {
                vfn.call(SyntaxViolation::NonUrlCodePoint)
            }
        }
    }
}

#[inline]
fn is_ascii_hex_digit(c: char) -> bool {
    matches!(c, 'a'...'f' | 'A'...'F' | '0'...'9')
}

// Non URL code points:
// U+0000 to U+0020 (space)
// " # % < > [ \ ] ^ ` { | }
// U+007F to U+009F
// surrogates
// U+FDD0 to U+FDEF
// Last two of each plane: U+__FFFE to U+__FFFF for __ in 00 to 10 hex
#[inline]
fn is_url_code_point(c: char) -> bool {
    matches!(c,
        'a'...'z' |
        'A'...'Z' |
        '0'...'9' |
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '-' |
        '.' | '/' | ':' | ';' | '=' | '?' | '@' | '_' | '~' |
        '\u{A0}'...'\u{D7FF}' | '\u{E000}'...'\u{FDCF}' | '\u{FDF0}'...'\u{FFFD}' |
        '\u{10000}'...'\u{1FFFD}' | '\u{20000}'...'\u{2FFFD}' |
        '\u{30000}'...'\u{3FFFD}' | '\u{40000}'...'\u{4FFFD}' |
        '\u{50000}'...'\u{5FFFD}' | '\u{60000}'...'\u{6FFFD}' |
        '\u{70000}'...'\u{7FFFD}' | '\u{80000}'...'\u{8FFFD}' |
        '\u{90000}'...'\u{9FFFD}' | '\u{A0000}'...'\u{AFFFD}' |
        '\u{B0000}'...'\u{BFFFD}' | '\u{C0000}'...'\u{CFFFD}' |
        '\u{D0000}'...'\u{DFFFD}' | '\u{E1000}'...'\u{EFFFD}' |
        '\u{F0000}'...'\u{FFFFD}' | '\u{100000}'...'\u{10FFFD}')
}

/// https://url.spec.whatwg.org/#c0-controls-and-space
#[inline]
fn c0_control_or_space(ch: char) -> bool {
    ch <= ' '  // U+0000 to U+0020
}

/// https://url.spec.whatwg.org/#ascii-alpha
#[inline]
pub fn ascii_alpha(ch: char) -> bool {
    matches!(ch, 'a'...'z' | 'A'...'Z')
}

#[inline]
pub fn to_u32(i: usize) -> ParseResult<u32> {
    if i <= ::std::u32::MAX as usize {
        Ok(i as u32)
    } else {
        Err(ParseError::Overflow)
    }
}
