//! Tools for blocking at a page-content level, including CSS selector-based filtering and content
//! script injection.
use serde::{Deserialize, Serialize};
use crate::utils::Hash;

use css_validation::{is_valid_css_selector, is_valid_css_style};

#[derive(Debug, PartialEq)]
pub enum CosmeticFilterError {
    PunycodeError,
    InvalidStyleSpecifier,
    UnsupportedSyntax,
    MissingSharp,
    InvalidCssStyle,
    InvalidCssSelector,
    GenericUnhide,
    GenericScriptInject,
    GenericStyle,
    DoubleNegation,
}

bitflags! {
    /// Boolean flags for cosmetic filter rules.
    #[derive(Serialize, Deserialize)]
    pub struct CosmeticFilterMask: u8 {
        const UNHIDE = 1 << 0;
        const SCRIPT_INJECT = 1 << 1;
        const IS_UNICODE = 1 << 2;
        const IS_CLASS_SELECTOR = 1 << 3;
        const IS_ID_SELECTOR = 1 << 4;

        // Careful with checking for NONE - will always match
        const NONE = 0;
    }
}

/// Struct representing a parsed cosmetic filter rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticFilter {
    pub entities: Option<Vec<Hash>>,
    pub hostnames: Option<Vec<Hash>>,
    pub mask: CosmeticFilterMask,
    pub not_entities: Option<Vec<Hash>>,
    pub not_hostnames: Option<Vec<Hash>>,
    pub raw_line: Option<String>,
    pub selector: String,
    pub style: Option<String>,
}

impl CosmeticFilter {
    /// Parses the contents of a cosmetic filter rule up to the `##` or `#@#` separator.
    ///
    /// On success, returns `Vec`s of hashes of all of the following comma separated items that
    /// were populated in the rule:
    ///
    ///    - `entities`: entity.*
    ///
    ///    - `not_entities`: ~entity.*
    ///
    ///    - `hostnames`: hostname
    ///
    ///    - `not_hostnames`: ~hostname
    ///
    /// This should only be called if `sharp_index` is greater than 0, in which case all four are
    /// guaranteed to be `None`.
    #[inline]
    fn parse_before_sharp(
        line: &str,
        sharp_index: usize,
        mask: &mut CosmeticFilterMask
    ) -> Result<(Option<Vec<Hash>>, Option<Vec<Hash>>, Option<Vec<Hash>>, Option<Vec<Hash>>), CosmeticFilterError> {
        let mut entities_vec = vec![];
        let mut not_entities_vec = vec![];
        let mut hostnames_vec = vec![];
        let mut not_hostnames_vec = vec![];

        let parts = line[0..sharp_index].split(',');
        for part in parts {
            let mut hostname = String::new();
            if part.is_ascii() {
                hostname.push_str(&part);
            } else {
                *mask |= CosmeticFilterMask::IS_UNICODE;
                match idna::domain_to_ascii(&part) {
                    Ok(x) => hostname.push_str(&x),
                    Err(_) => return Err(CosmeticFilterError::PunycodeError),
                }
            }
            let negation = hostname.starts_with('~');
            let entity = hostname.ends_with(".*");
            let start = if negation {
                1
            } else {
                0
            };
            let end = if entity {
                hostname.len() - 2
            } else {
                hostname.len()
            };
            let hash = crate::utils::fast_hash(&hostname[start..end]);
            match (negation, entity) {
                (true, true) => not_entities_vec.push(hash),
                (true, false) => not_hostnames_vec.push(hash),
                (false, true) => entities_vec.push(hash),
                (false, false) => hostnames_vec.push(hash),
            }
        };

        /// Sorts `vec` and wraps it in `Some` if it's not empty, or returns `None` if it is.
        #[inline]
        fn sorted_or_none<T: std::cmp::Ord>(mut vec: Vec<T>) -> Option<Vec<T>> {
            if!vec.is_empty() {
                vec.sort();
                Some(vec)
            } else {
                None
            }
        }

        let entities = sorted_or_none(entities_vec);
        let hostnames = sorted_or_none(hostnames_vec);
        let not_entities = sorted_or_none(not_entities_vec);
        let not_hostnames = sorted_or_none(not_hostnames_vec);

        Ok((entities, not_entities, hostnames, not_hostnames))
    }

    /// Parses the contents of a cosmetic filter rule following the `##` or `#@#` separator.
    ///
    /// On success, updates the contents of `selector` and `style` according to the rule.
    ///
    /// This should only be called if the rule part after the separator has been confirmed not to
    /// be a script injection rule using `+js()`.
    #[inline]
    fn parse_after_sharp_nonscript<'a>(
        line: &'a str,
        suffix_start_index: usize,
        selector: &mut &'a str,
        style: &mut Option<String>
    ) -> Result<(), CosmeticFilterError> {
        let mut index_after_colon = suffix_start_index;
        while let Some(colon_index) = line[index_after_colon..].find(':') {
            let colon_index = colon_index + index_after_colon;
            index_after_colon = colon_index + 1;
            let content_after_colon = &line[index_after_colon..];
            if content_after_colon.starts_with("style") {
                if content_after_colon.chars().nth(5) == Some('(') && content_after_colon.chars().nth(content_after_colon.len() - 1) == Some(')') {
                    *selector = &line[suffix_start_index..colon_index];
                    *style = Some(content_after_colon[6..content_after_colon.len()-1].to_string());
                } else {
                    return Err(CosmeticFilterError::InvalidStyleSpecifier);
                }
            } else if content_after_colon.starts_with("-abp-")
            || content_after_colon.starts_with("contains")
            || content_after_colon.starts_with("has")
            || content_after_colon.starts_with("if")
            || content_after_colon.starts_with("if-not")
            || content_after_colon.starts_with("matches-css")
            || content_after_colon.starts_with("matches-css-after")
            || content_after_colon.starts_with("matches-css-before")
            || content_after_colon.starts_with("properties")
            || content_after_colon.starts_with("subject")
            || content_after_colon.starts_with("xpath")
            {
                return Err(CosmeticFilterError::UnsupportedSyntax);
            }
        }
        Ok(())
    }

    /// Parse the rule in `line` into a `CosmeticFilter`. If `debug` is true, the original rule
    /// will be reported in the resulting `CosmeticFilter` struct as well.
    pub fn parse(line: &str, debug: bool) -> Result<CosmeticFilter, CosmeticFilterError> {
        let mut mask = CosmeticFilterMask::NONE;
        if let Some(sharp_index) = line.find('#') {
            let after_sharp_index = sharp_index + 1;
            let mut suffix_start_index = after_sharp_index + 1;

            if line[after_sharp_index..].starts_with("@") {
                if sharp_index == 0 {
                    return Err(CosmeticFilterError::GenericUnhide);
                }
                mask |= CosmeticFilterMask::UNHIDE;
                suffix_start_index += 1;
            }

            // 1 - sharp_index
            // 2 - after_sharp_index
            // 3 - suffix_start_index
            //
            // hostnames##selector
            //          123
            //
            // hostnames#@#selector
            //          12 3

            let (entities, not_entities, hostnames, not_hostnames) = if sharp_index > 0 {
                CosmeticFilter::parse_before_sharp(line, sharp_index, &mut mask)?
            } else {
                (None, None, None, None)
            };

            let mut selector = &line[suffix_start_index..];
            let mut style = None;
            if line.len() - suffix_start_index > 4 && line[suffix_start_index..].starts_with("+js(") {
                if sharp_index == 0 {
                    return Err(CosmeticFilterError::GenericScriptInject);
                }
                mask |= CosmeticFilterMask::SCRIPT_INJECT;
                selector = &line[suffix_start_index + 4..line.len() - 1];
            } else {
                CosmeticFilter::parse_after_sharp_nonscript(line, suffix_start_index, &mut selector, &mut style)?;
            }

            if !mask.contains(CosmeticFilterMask::SCRIPT_INJECT) && !is_valid_css_selector(selector) {
                return Err(CosmeticFilterError::InvalidCssSelector);
            } else if let Some(ref style) = style {
                if !is_valid_css_style(style) {
                    return Err(CosmeticFilterError::InvalidCssStyle);
                } else if sharp_index == 0 {
                    return Err(CosmeticFilterError::GenericStyle);
                }
            }

            if (not_entities.is_some() || not_hostnames.is_some()) && mask.contains(CosmeticFilterMask::UNHIDE) {
                return Err(CosmeticFilterError::DoubleNegation);
            }

            if !selector.is_ascii() {
                mask |= CosmeticFilterMask::IS_UNICODE;
            }

            if !mask.contains(CosmeticFilterMask::SCRIPT_INJECT) {
                if selector.starts_with('.') && is_simple_selector(selector) {
                    mask |= CosmeticFilterMask::IS_CLASS_SELECTOR;
                } else if selector.starts_with('#') && is_simple_selector(selector) {
                    mask |= CosmeticFilterMask::IS_ID_SELECTOR;
                }
            }

            Ok(CosmeticFilter {
                entities,
                hostnames,
                mask,
                not_entities,
                not_hostnames,
                raw_line: if debug {
                    Some(String::from(line))
                } else {
                    None
                },
                selector: String::from(selector),
                style,
            })
        } else {
            Err(CosmeticFilterError::MissingSharp)
        }
    }
}

mod css_validation {
    //! Methods for validating CSS selectors and style rules extracted from cosmetic filter rules.
    use cssparser::ParserInput;
    use cssparser::Parser;
    use selectors::parser::Selector;

    use std::fmt::{Display, Formatter, Error};
    use core::fmt::{Write, Result as FmtResult};

    pub fn is_valid_css_selector(selector: &str) -> bool {
        let mut pi = ParserInput::new(selector);
        let mut parser = Parser::new(&mut pi);
        let r = Selector::parse(&SelectorParseImpl, &mut parser);
        r.is_ok()
    }

    pub fn is_valid_css_style(style: &str) -> bool {
        if style.contains('\\') {
            return false;
        }
        if style.contains("url(") {
            return false;
        }
        true
    }

    struct SelectorParseImpl;

    impl<'i> selectors::parser::Parser<'i> for SelectorParseImpl {
        type Impl = SelectorImpl;
        type Error = selectors::parser::SelectorParseErrorKind<'i>;
    }

    /// The `selectors` library requires an object that implements `SelectorImpl` to store data
    /// about a parsed selector. For performance, the actual content of parsed selectors is
    /// discarded as much as possible - it only matters whether the returned `Result` is `Ok` or
    /// `Err`.
    #[derive(Debug, Clone)]
    struct SelectorImpl;

    impl selectors::parser::SelectorImpl for SelectorImpl {
        type ExtraMatchingData = ();
        type AttrValue = DummyValue;
        type Identifier = DummyValue;
        type ClassName = DummyValue;
        type LocalName = String;
        type NamespaceUrl = String;
        type NamespacePrefix = DummyValue;
        type BorrowedNamespaceUrl = String;
        type BorrowedLocalName = String;
        type NonTSPseudoClass = NonTSPseudoClass;
        type PseudoElement = PseudoElement;
    }

    /// For performance, individual fields of parsed selectors is discarded. Instead, they are
    /// parsed into a `DummyValue` with no fields.
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct DummyValue;

    impl Display for DummyValue {
        fn fmt(&self, _: &mut Formatter) -> Result<(), Error> { Ok(()) }
    }

    impl<'a> From<&'a str> for DummyValue {
        fn from(_: &'a str) -> Self { DummyValue }
    }

    /// Dummy struct for non-tree-structural pseudo-classes.
    #[derive(Clone, PartialEq, Eq)]
    struct NonTSPseudoClass;

    impl selectors::parser::NonTSPseudoClass for NonTSPseudoClass {
        type Impl = SelectorImpl;
        fn is_active_or_hover(&self) -> bool { false }
    }

    impl cssparser::ToCss for NonTSPseudoClass {
        fn to_css<W: Write>(&self, _: &mut W) -> FmtResult { Ok(()) }
    }

    /// Dummy struct for pseudo-elements.
    #[derive(Clone, PartialEq, Eq)]
    struct PseudoElement;

    impl selectors::parser::PseudoElement for PseudoElement {
        type Impl = SelectorImpl;

        fn supports_pseudo_class(&self, _pseudo_class: &NonTSPseudoClass) -> bool { true }

        fn valid_after_slotted(&self) -> bool { true }
    }

    impl cssparser::ToCss for PseudoElement {
        fn to_css<W: Write>(&self, _dest: &mut W) -> FmtResult { Ok(()) }
    }

    #[test]
    fn bad_selector_inputs() {
        assert!(!is_valid_css_selector(r#"rm -rf ./*"#));
        assert!(!is_valid_css_selector(r#"javascript:alert("hacked")"#));
        assert!(!is_valid_css_selector(r#"This is not a CSS selector."#));
        assert!(!is_valid_css_selector(r#"./malware.sh"#));
        assert!(!is_valid_css_selector(r#"https://safesite.ru"#));
        assert!(!is_valid_css_selector(r#"(function(){var e=60;return String.fromCharCode(e.charCodeAt(0))})();"#));
        assert!(!is_valid_css_selector(r#"#!/usr/bin/sh"#));
    }
}

/// A selector is a simple selector if it is an id or class selector, optionally followed by a
/// square-bracketed attribute selector or another ` >`, ` +`, ` .`, or ` #` rule. In each of these
/// cases, the rule would be indexed by the first class or id specified.
///
/// This should only be called after verifying that the first character of the selector is a `#` or
/// a `.`.
fn is_simple_selector(selector: &str) -> bool {
    for (i, c) in selector.chars().enumerate().skip(1) {
        if !(c == '-'
            || c == '_'
            || (c >= '0' && c <= '9')
            || (c >= 'A' && c <= 'Z')
            || (c >= 'a' && c <= 'z'))
        {
            if i < selector.len() - 1 {
                // Unwrap is safe here because of the range check above
                let next = selector.chars().nth(i + 1).unwrap();
                if c == '['
                    || (c == ' '
                        && (next == '>'
                            || next == '+'
                            || next == '~'
                            || next == '.'
                            || next == '#'))
                {
                    return true;
                }
            }
            return false;
        }
    }
    true
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    /// An easily modified summary of a `CosmeticFilter` rule to be used in tests.
    #[derive(Debug, PartialEq)]
    struct CosmeticFilterBreakdown {
        entities: Option<Vec<Hash>>,
        hostnames: Option<Vec<Hash>>,
        not_entities: Option<Vec<Hash>>,
        not_hostnames: Option<Vec<Hash>>,
        selector: String,
        style: Option<String>,

        unhide: bool,
        script_inject: bool,
        is_unicode: bool,
        is_class_selector: bool,
        is_id_selector: bool,
    }

    impl From<&CosmeticFilter> for CosmeticFilterBreakdown {
        fn from(filter: &CosmeticFilter) -> CosmeticFilterBreakdown {
            CosmeticFilterBreakdown {
                entities: filter.entities.as_ref().cloned(),
                hostnames: filter.hostnames.as_ref().cloned(),
                not_entities: filter.not_entities.as_ref().cloned(),
                not_hostnames: filter.not_hostnames.as_ref().cloned(),
                selector: filter.selector.clone(),
                style: filter.style.as_ref().cloned(),

                unhide: filter.mask.contains(CosmeticFilterMask::UNHIDE),
                script_inject: filter.mask.contains(CosmeticFilterMask::SCRIPT_INJECT),
                is_unicode: filter.mask.contains(CosmeticFilterMask::IS_UNICODE),
                is_class_selector: filter.mask.contains(CosmeticFilterMask::IS_CLASS_SELECTOR),
                is_id_selector: filter.mask.contains(CosmeticFilterMask::IS_ID_SELECTOR),
            }
        }
    }

    impl Default for CosmeticFilterBreakdown {
        fn default() -> Self {
            CosmeticFilterBreakdown {
                entities: None,
                hostnames: None,
                not_entities: None,
                not_hostnames: None,
                selector: "".to_string(),
                style: None,

                unhide: false,
                script_inject: false,
                is_unicode: false,
                is_class_selector: false,
                is_id_selector: false,
            }
        }
    }

    /// Asserts that `rule` parses into a `CosmeticFilter` equivalent to the summary provided by
    /// `expected`.
    fn check_parse_result(rule: &str, expected: CosmeticFilterBreakdown) {
        let filter: CosmeticFilterBreakdown = (&CosmeticFilter::parse(rule, false).unwrap()).into();
        assert_eq!(expected, filter);
    }

    #[test]
    fn simple_selectors() {
        check_parse_result(
            "##div.popup",
            CosmeticFilterBreakdown {
                selector: "div.popup".to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            "###selector",
            CosmeticFilterBreakdown {
                selector: "#selector".to_string(),
                is_id_selector: true,
                ..Default::default()
            }
        );
        check_parse_result(
            "##.selector",
            CosmeticFilterBreakdown {
                selector: ".selector".to_string(),
                is_class_selector: true,
                ..Default::default()
            }
        );
        check_parse_result(
            "##a[href=\"foo.com\"]",
            CosmeticFilterBreakdown {
                selector: "a[href=\"foo.com\"]".to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            "##[href=\"foo.com\"]",
            CosmeticFilterBreakdown {
                selector: "[href=\"foo.com\"]".to_string(),
                ..Default::default()
            }
        );
    }

    /// Produces a sorted vec of the hashes of all the given domains.
    ///
    /// For convenience, the return value is wrapped in a `Some()` to be consumed by a
    /// `CosmeticFilterBreakdown`.
    fn sort_hash_domains(domains: Vec<&str>) -> Option<Vec<Hash>> {
        let mut hashes: Vec<_> = domains.iter().map(|d| crate::utils::fast_hash(d)).collect();
        hashes.sort();
        Some(hashes)
    }

    #[test]
    fn hostnames() {
        check_parse_result(
            r#"u00p.com##div[class^="adv-box"]"#,
            CosmeticFilterBreakdown {
                selector: r#"div[class^="adv-box"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["u00p.com"]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"distractify.com##div[class*="AdInArticle"]"#,
            CosmeticFilterBreakdown {
                selector: r#"div[class*="AdInArticle"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["distractify.com"]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"soundtrackcollector.com,the-numbers.com##a[href^="http://affiliates.allposters.com/"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href^="http://affiliates.allposters.com/"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["soundtrackcollector.com", "the-numbers.com"]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"thelocal.at,thelocal.ch,thelocal.de,thelocal.dk,thelocal.es,thelocal.fr,thelocal.it,thelocal.no,thelocal.se##div[class*="-widget"]"#,
            CosmeticFilterBreakdown {
                selector: r#"div[class*="-widget"]"#.to_string(),
                hostnames: sort_hash_domains(vec![
                     "thelocal.at",
                     "thelocal.ch",
                     "thelocal.de",
                     "thelocal.dk",
                     "thelocal.es",
                     "thelocal.fr",
                     "thelocal.it",
                     "thelocal.no",
                     "thelocal.se",
                ]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"base64decode.org,base64encode.org,beautifyjson.org,minifyjson.org,numgen.org,pdfmrg.com,pdfspl.com,prettifycss.com,pwdgen.org,strlength.com,strreverse.com,uglifyjs.net,urldecoder.org##div[class^="banner_"]"#,
            CosmeticFilterBreakdown {
                selector: r#"div[class^="banner_"]"#.to_string(),
                hostnames: sort_hash_domains(vec![
                     "base64decode.org",
                     "base64encode.org",
                     "beautifyjson.org",
                     "minifyjson.org",
                     "numgen.org",
                     "pdfmrg.com",
                     "pdfspl.com",
                     "prettifycss.com",
                     "pwdgen.org",
                     "strlength.com",
                     "strreverse.com",
                     "uglifyjs.net",
                     "urldecoder.org"
                ]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"adforum.com,alliednews.com,americustimesrecorder.com,andovertownsman.com,athensreview.com,batesvilleheraldtribune.com,bdtonline.com,channel24.pk,chickashanews.com,claremoreprogress.com,cleburnetimesreview.com,clintonherald.com,commercejournal.com,commercial-news.com,coopercrier.com,cordeledispatch.com,corsicanadailysun.com,crossville-chronicle.com,cullmantimes.com,dailyiowegian.com,dailyitem.com,daltondailycitizen.com,derrynews.com,duncanbanner.com,eagletribune.com,edmondsun.com,effinghamdailynews.com,enewscourier.com,enidnews.com,farmtalknewspaper.com,fayettetribune.com,flasharcade.com,flashgames247.com,flyergroup.com,foxsportsasia.com,gainesvilleregister.com,gloucestertimes.com,goshennews.com,greensburgdailynews.com,heraldbanner.com,heraldbulletin.com,hgazette.com,homemagonline.com,itemonline.com,jacksonvilleprogress.com,jerusalemonline.com,joplinglobe.com,journal-times.com,journalexpress.net,kexp.org,kokomotribune.com,lockportjournal.com,mankatofreepress.com,mcalesternews.com,mccrearyrecord.com,mcleansborotimesleader.com,meadvilletribune.com,meridianstar.com,mineralwellsindex.com,montgomery-herald.com,mooreamerican.com,moultrieobserver.com,muskogeephoenix.com,ncnewsonline.com,newburyportnews.com,newsaegis.com,newsandtribune.com,niagara-gazette.com,njeffersonnews.com,normantranscript.com,opposingviews.com,orangeleader.com,oskaloosa.com,ottumwacourier.com,outlookmoney.com,palestineherald.com,panews.com,paulsvalleydailydemocrat.com,pellachronicle.com,pharostribune.com,pressrepublican.com,pryordailytimes.com,randolphguide.com,record-eagle.com,register-herald.com,register-news.com,reporter.net,rockwallheraldbanner.com,roysecityheraldbanner.com,rushvillerepublican.com,salemnews.com,sentinel-echo.com,sharonherald.com,shelbyvilledailyunion.com,siteslike.com,standardmedia.co.ke,starbeacon.com,stwnewspress.com,suwanneedemocrat.com,tahlequahdailypress.com,theadanews.com,theawesomer.com,thedailystar.com,thelandonline.com,themoreheadnews.com,thesnaponline.com,tiftongazette.com,times-news.com,timesenterprise.com,timessentinel.com,timeswv.com,tonawanda-news.com,tribdem.com,tribstar.com,unionrecorder.com,valdostadailytimes.com,washtimesherald.com,waurikademocrat.com,wcoutlook.com,weatherforddemocrat.com,woodwardnews.net,wrestlinginc.com##div[style="width:300px; height:250px;"]"#,
            CosmeticFilterBreakdown {
                selector: r#"div[style="width:300px; height:250px;"]"#.to_string(),
                hostnames: sort_hash_domains(vec![
                    "adforum.com",
                    "alliednews.com",
                    "americustimesrecorder.com",
                    "andovertownsman.com",
                    "athensreview.com",
                    "batesvilleheraldtribune.com",
                    "bdtonline.com",
                    "channel24.pk",
                    "chickashanews.com",
                    "claremoreprogress.com",
                    "cleburnetimesreview.com",
                    "clintonherald.com",
                    "commercejournal.com",
                    "commercial-news.com",
                    "coopercrier.com",
                    "cordeledispatch.com",
                    "corsicanadailysun.com",
                    "crossville-chronicle.com",
                    "cullmantimes.com",
                    "dailyiowegian.com",
                    "dailyitem.com",
                    "daltondailycitizen.com",
                    "derrynews.com",
                    "duncanbanner.com",
                    "eagletribune.com",
                    "edmondsun.com",
                    "effinghamdailynews.com",
                    "enewscourier.com",
                    "enidnews.com",
                    "farmtalknewspaper.com",
                    "fayettetribune.com",
                    "flasharcade.com",
                    "flashgames247.com",
                    "flyergroup.com",
                    "foxsportsasia.com",
                    "gainesvilleregister.com",
                    "gloucestertimes.com",
                    "goshennews.com",
                    "greensburgdailynews.com",
                    "heraldbanner.com",
                    "heraldbulletin.com",
                    "hgazette.com",
                    "homemagonline.com",
                    "itemonline.com",
                    "jacksonvilleprogress.com",
                    "jerusalemonline.com",
                    "joplinglobe.com",
                    "journal-times.com",
                    "journalexpress.net",
                    "kexp.org",
                    "kokomotribune.com",
                    "lockportjournal.com",
                    "mankatofreepress.com",
                    "mcalesternews.com",
                    "mccrearyrecord.com",
                    "mcleansborotimesleader.com",
                    "meadvilletribune.com",
                    "meridianstar.com",
                    "mineralwellsindex.com",
                    "montgomery-herald.com",
                    "mooreamerican.com",
                    "moultrieobserver.com",
                    "muskogeephoenix.com",
                    "ncnewsonline.com",
                    "newburyportnews.com",
                    "newsaegis.com",
                    "newsandtribune.com",
                    "niagara-gazette.com",
                    "njeffersonnews.com",
                    "normantranscript.com",
                    "opposingviews.com",
                    "orangeleader.com",
                    "oskaloosa.com",
                    "ottumwacourier.com",
                    "outlookmoney.com",
                    "palestineherald.com",
                    "panews.com",
                    "paulsvalleydailydemocrat.com",
                    "pellachronicle.com",
                    "pharostribune.com",
                    "pressrepublican.com",
                    "pryordailytimes.com",
                    "randolphguide.com",
                    "record-eagle.com",
                    "register-herald.com",
                    "register-news.com",
                    "reporter.net",
                    "rockwallheraldbanner.com",
                    "roysecityheraldbanner.com",
                    "rushvillerepublican.com",
                    "salemnews.com",
                    "sentinel-echo.com",
                    "sharonherald.com",
                    "shelbyvilledailyunion.com",
                    "siteslike.com",
                    "standardmedia.co.ke",
                    "starbeacon.com",
                    "stwnewspress.com",
                    "suwanneedemocrat.com",
                    "tahlequahdailypress.com",
                    "theadanews.com",
                    "theawesomer.com",
                    "thedailystar.com",
                    "thelandonline.com",
                    "themoreheadnews.com",
                    "thesnaponline.com",
                    "tiftongazette.com",
                    "times-news.com",
                    "timesenterprise.com",
                    "timessentinel.com",
                    "timeswv.com",
                    "tonawanda-news.com",
                    "tribdem.com",
                    "tribstar.com",
                    "unionrecorder.com",
                    "valdostadailytimes.com",
                    "washtimesherald.com",
                    "waurikademocrat.com",
                    "wcoutlook.com",
                    "weatherforddemocrat.com",
                    "woodwardnews.net",
                    "wrestlinginc.com",
                ]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn href() {
        check_parse_result(
            r#"##a[href$="/vghd.shtml"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href$="/vghd.shtml"]"#.to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"##a[href*=".adk2x.com/"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href*=".adk2x.com/"]"#.to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"##a[href^="//40ceexln7929.com/"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href^="//40ceexln7929.com/"]"#.to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"##a[href*=".trust.zone"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href*=".trust.zone"]"#.to_string(),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"tf2maps.net##a[href="http://forums.tf2maps.net/payments.php"]"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href="http://forums.tf2maps.net/payments.php"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["tf2maps.net"]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"rarbg.to,rarbg.unblockall.org,rarbgaccess.org,rarbgmirror.com,rarbgmirror.org,rarbgmirror.xyz,rarbgproxy.com,rarbgproxy.org,rarbgunblock.com##a[href][target="_blank"] > button"#,
            CosmeticFilterBreakdown {
                selector: r#"a[href][target="_blank"] > button"#.to_string(),
                hostnames: sort_hash_domains(vec![
                     "rarbg.to",
                     "rarbg.unblockall.org",
                     "rarbgaccess.org",
                     "rarbgmirror.com",
                     "rarbgmirror.org",
                     "rarbgmirror.xyz",
                     "rarbgproxy.com",
                     "rarbgproxy.org",
                     "rarbgunblock.com",
                ]),
                ..Default::default()
            }
        );
    }

    #[test]
    fn injected_scripts() {
        check_parse_result(
            r#"hentaifr.net,jeu.info,tuxboard.com,xstory-fr.com##+js(goyavelab-defuser.js)"#,
            CosmeticFilterBreakdown {
                selector: r#"goyavelab-defuser.js"#.to_string(),
                hostnames: sort_hash_domains(vec![
                    "hentaifr.net",
                    "jeu.info",
                    "tuxboard.com",
                    "xstory-fr.com",
                ]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"haus-garten-test.de,sozialversicherung-kompetent.de##+js(set-constant.js, Object.keys, trueFunc)"#,
            CosmeticFilterBreakdown {
                selector: r#"set-constant.js, Object.keys, trueFunc"#.to_string(),
                hostnames: sort_hash_domains(vec!["haus-garten-test.de", "sozialversicherung-kompetent.de"]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"airliners.de,auszeit.bio,autorevue.at,clever-tanken.de,fanfiktion.de,finya.de,frag-mutti.de,frustfrei-lernen.de,fussballdaten.de,gameswelt.*,liga3-online.de,lz.de,mt.de,psychic.de,rimondo.com,spielen.de,weltfussball.at,weristdeinfreund.de##+js(abort-current-inline-script.js, Number.isNaN)"#,
            CosmeticFilterBreakdown {
                selector: r#"abort-current-inline-script.js, Number.isNaN"#.to_string(),
                hostnames: sort_hash_domains(vec![
                    "airliners.de",
                    "auszeit.bio",
                    "autorevue.at",
                    "clever-tanken.de",
                    "fanfiktion.de",
                    "finya.de",
                    "frag-mutti.de",
                    "frustfrei-lernen.de",
                    "fussballdaten.de",
                    "liga3-online.de",
                    "lz.de",
                    "mt.de",
                    "psychic.de",
                    "rimondo.com",
                    "spielen.de",
                    "weltfussball.at",
                    "weristdeinfreund.de",
                ]),
                entities: sort_hash_domains(vec![
                    "gameswelt",
                ]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"prad.de##+js(abort-on-property-read.js, document.cookie)"#,
            CosmeticFilterBreakdown {
                selector: r#"abort-on-property-read.js, document.cookie"#.to_string(),
                hostnames: sort_hash_domains(vec!["prad.de"]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"computerbild.de##+js(abort-on-property-read.js, Date.prototype.toUTCString)"#,
            CosmeticFilterBreakdown {
                selector: r#"abort-on-property-read.js, Date.prototype.toUTCString"#.to_string(),
                hostnames: sort_hash_domains(vec!["computerbild.de"]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"computerbild.de##+js(setTimeout-defuser.js, ())return)"#,
            CosmeticFilterBreakdown {
                selector: r#"setTimeout-defuser.js, ())return"#.to_string(),
                hostnames: sort_hash_domains(vec!["computerbild.de"]),
                script_inject: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn entities() {
        check_parse_result(
            r#"monova.*##+js(nowebrtc.js)"#,
            CosmeticFilterBreakdown {
                selector: r#"nowebrtc.js"#.to_string(),
                entities: sort_hash_domains(vec!["monova"]),
                script_inject: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"monova.*##tr.success.desktop"#,
            CosmeticFilterBreakdown {
                selector: r#"tr.success.desktop"#.to_string(),
                entities: sort_hash_domains(vec!["monova"]),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"monova.*#@#script + [class] > [class]:first-child"#,
            CosmeticFilterBreakdown {
                selector: r#"script + [class] > [class]:first-child"#.to_string(),
                entities: sort_hash_domains(vec!["monova"]),
                unhide: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"adshort.im,adsrt.*#@#[id*="ScriptRoot"]"#,
            CosmeticFilterBreakdown {
                selector: r#"[id*="ScriptRoot"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["adshort.im"]),
                entities: sort_hash_domains(vec!["adsrt"]),
                unhide: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"downloadsource.*##.date:not(dt):style(display: block !important;)"#,
            CosmeticFilterBreakdown {
                selector: r#".date:not(dt)"#.to_string(),
                entities: sort_hash_domains(vec!["downloadsource"]),
                style: Some("display: block !important;".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn styles() {
        check_parse_result(
            r#"chip.de##.video-wrapper > video[style]:style(display:block!important;padding-top:0!important;)"#,
            CosmeticFilterBreakdown {
                selector: r#".video-wrapper > video[style]"#.to_string(),
                hostnames: sort_hash_domains(vec!["chip.de"]),
                style: Some("display:block!important;padding-top:0!important;".into()),
                is_class_selector: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"allmusic.com##.advertising.medium-rectangle:style(min-height: 1px !important;)"#,
            CosmeticFilterBreakdown {
                selector: r#".advertising.medium-rectangle"#.to_string(),
                hostnames: sort_hash_domains(vec!["allmusic.com"]),
                style: Some("min-height: 1px !important;".into()),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"quora.com##.signup_wall_prevent_scroll .SiteHeader,.signup_wall_prevent_scroll .LoggedOutFooter,.signup_wall_prevent_scroll .ContentWrapper:style(filter: none !important;)"#,
            CosmeticFilterBreakdown {
                selector: r#".signup_wall_prevent_scroll .SiteHeader,.signup_wall_prevent_scroll .LoggedOutFooter,.signup_wall_prevent_scroll .ContentWrapper"#.to_string(),
                hostnames: sort_hash_domains(vec!["quora.com"]),
                style: Some("filter: none !important;".into()),
                is_class_selector: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"imdb.com##body#styleguide-v2:style(background-color: #e3e2dd !important; background-image: none !important;)"#,
            CosmeticFilterBreakdown {
                selector: r#"body#styleguide-v2"#.to_string(),
                hostnames: sort_hash_domains(vec!["imdb.com"]),
                style: Some("background-color: #e3e2dd !important; background-image: none !important;".into()),
                ..Default::default()
            }
        );
        check_parse_result(
            r#"streamcloud.eu###login > div[style^="width"]:style(display: block !important)"#,
            CosmeticFilterBreakdown {
                selector: r#"#login > div[style^="width"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["streamcloud.eu"]),
                style: Some("display: block !important".into()),
                is_id_selector: true,
                ..Default::default()
            }
        );
        check_parse_result(
            r#"moonbit.co.in,moondoge.co.in,moonliteco.in##[src^="//coinad.com/ads/"]:style(visibility: collapse !important)"#,
            CosmeticFilterBreakdown {
                selector: r#"[src^="//coinad.com/ads/"]"#.to_string(),
                hostnames: sort_hash_domains(vec!["moonbit.co.in", "moondoge.co.in", "moonliteco.in"]),
                style: Some("visibility: collapse !important".into()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn unicode() {
        check_parse_result(
            "###неделя",
            CosmeticFilterBreakdown {
                selector: "#неделя".to_string(),
                is_unicode: true,
                ..Default::default()
            }
        );
        check_parse_result(
            "неlloworlд.com#@##week",
            CosmeticFilterBreakdown {
                selector: "#week".to_string(),
                hostnames: sort_hash_domains(vec!["xn--lloworl-5ggb3f.com"]),
                is_unicode: true,
                is_id_selector: true,
                unhide: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn unsupported() {
        assert!(CosmeticFilter::parse("yandex.*##.serp-item:if(:scope > div.organic div.organic__subtitle:matches-css-after(content: /[Рр]еклама/))", false).is_err());
        assert!(CosmeticFilter::parse(r#"facebook.com,facebookcorewwwi.onion##.ego_column:if(a[href^="/campaign/landing"])"#, false).is_err());
        assert!(CosmeticFilter::parse(r#"thedailywtf.com##.article-body > div:has(a[href*="utm_medium"])"#, false).is_err());
        assert!(CosmeticFilter::parse(r#"readcomiconline.to##^script:has-text(this[atob)"#, false).is_err());
        assert!(CosmeticFilter::parse("twitter.com##article:has-text(/Promoted|Gesponsert|Реклама|Promocionado/):xpath(../..)", false).is_err());
    }
}
