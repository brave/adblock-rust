//! Provides behavior related to cosmetic filtering - that is, modifying a page's contents after
//! it's been loaded into a browser. This is primarily used to hide or clean up unwanted page
//! elements that are served inline with the rest of the first-party content from a page, but can
//! also be used to inject JavaScript "scriptlets" that intercept and modify the behavior of
//! scripts on the page at runtime.
//!
//! The primary API exposed by this module is the `CosmeticFilterCache` struct, which stores
//! cosmetic filters and allows them to be queried efficiently at runtime for any which may be
//! relevant to a particular page.

use crate::filters::cosmetic::{
    CosmeticFilter, CosmeticFilterAction, CosmeticFilterMask, CosmeticFilterOperator,
};
use crate::filters::fb_network::flat::fb;
use crate::filters::fb_network::FilterDataContextRef;
use crate::flatbuffers::containers::flat_map::{FlatMapBuilder, FlatMapView};
use crate::flatbuffers::containers::flat_multimap::{
    FlatMapStringView, FlatMultiMapBuilder, FlatMultiMapView,
};

use crate::flatbuffers::containers::flat_serialize::{
    serialize_vec_opt, FlatBuilder, FlatSerialize,
};
use crate::flatbuffers::containers::flat_set::FlatSetView;

use crate::resources::{PermissionMask, ResourceStorage};
use crate::utils::Hash;

use std::collections::{HashMap, HashSet};

use flatbuffers::WIPOffset;
use memchr::memchr as find_char;
use serde::{Deserialize, Serialize};

/// Encodes permission bits in the first byte of a script string
/// Returns the script with permission byte prepended
fn encode_script_with_permission(script: String, permission: PermissionMask) -> String {
    let mut encoded = String::with_capacity(script.len() + 1);
    encoded.push(permission.to_bits() as char);
    encoded.push_str(&script);
    encoded
}

/// Decodes permission bits from the first byte of a script string
/// Returns (permission, script) tuple
fn decode_script_with_permission(encoded_script: &str) -> (PermissionMask, &str) {
    if encoded_script.is_empty() {
        return (PermissionMask::default(), encoded_script);
    }

    let first_char = encoded_script.chars().next().unwrap();
    let permission_bits = first_char as u8;
    let permission = PermissionMask::from_bits(permission_bits);
    let script = &encoded_script[first_char.len_utf8()..];
    (permission, script)
}

/// Contains cosmetic filter information intended to be used on a particular URL.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct UrlSpecificResources {
    /// `hide_selectors` is a set of any CSS selector on the page that should be hidden, i.e.
    /// styled as `{ display: none !important; }`.
    pub hide_selectors: HashSet<String>,
    /// Set of JSON-encoded procedural filters or filters with an action.
    pub procedural_actions: HashSet<String>,
    /// `exceptions` is a set of any class or id CSS selectors that should not have generic rules
    /// applied. In practice, these should be passed to `class_id_stylesheet` and not used
    /// otherwise.
    pub exceptions: HashSet<String>,
    /// `injected_script` is the Javascript code for any scriptlets that should be injected into
    /// the page.
    pub injected_script: String,
    /// `generichide` is set to true if there is a corresponding `$generichide` exception network
    /// filter. If so, the page should not query for additional generic rules using
    /// `hidden_class_id_selectors`.
    pub generichide: bool,
}

impl UrlSpecificResources {
    pub fn empty() -> Self {
        Self {
            hide_selectors: HashSet::new(),
            procedural_actions: HashSet::new(),
            exceptions: HashSet::new(),
            injected_script: String::new(),
            generichide: false,
        }
    }
}

/// The main engine driving cosmetic filtering.
///
/// There are two primary methods that should be considered when using this in a browser:
/// `hidden_class_id_selectors`, and `url_cosmetic_resources`.
///
/// Note that cosmetic filtering is imprecise and that this structure is intenionally designed for
/// efficient querying in the context of a browser, optimizing for low memory usage in the page
/// context and good performance. It is *not* designed to provide a 100% accurate report of what
/// will be blocked on any particular page, although when used correctly, all provided rules and
/// scriptlets should be safe to apply.
pub(crate) struct CosmeticFilterCache {
    filter_data_context: FilterDataContextRef,
}

/// Accumulates hostname-specific rules for a single domain before building HostnameSpecificRules
/// Note: hide and inject_script are now handled separately at the top level
#[derive(Default)]
struct HostnameRule {
    unhide: Vec<String>,
    uninject_script: Vec<String>,
    procedural_action: Vec<String>,
    procedural_action_exception: Vec<String>,
}

impl<'a, B: FlatBuilder<'a>> FlatSerialize<'a, B> for HostnameRule {
    type Output = WIPOffset<fb::HostnameSpecificRules<'a>>;

    fn serialize(
        value: Self,
        builder: &mut B,
    ) -> flatbuffers::WIPOffset<fb::HostnameSpecificRules<'a>> {
        let unhide = serialize_vec_opt(value.unhide, builder);
        let uninject_script = serialize_vec_opt(value.uninject_script, builder);
        let procedural_action = serialize_vec_opt(value.procedural_action, builder);
        let procedural_action_exception =
            serialize_vec_opt(value.procedural_action_exception, builder);

        fb::HostnameSpecificRules::create(
            builder.raw_builder(),
            &fb::HostnameSpecificRulesArgs {
                unhide,
                uninject_script,
                procedural_action,
                procedural_action_exception,
            },
        )
    }
}

#[derive(Default)]
pub(crate) struct CosmeticFilterCacheBuilder {
    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    misc_generic_selectors: HashSet<String>,
    complex_class_rules: FlatMultiMapBuilder<String, String>,
    complex_id_rules: FlatMultiMapBuilder<String, String>,

    hostname_hide: FlatMultiMapBuilder<Hash, String>,
    hostname_inject_script: FlatMultiMapBuilder<Hash, String>,

    specific_rules: HashMap<Hash, HostnameRule>,
}

impl CosmeticFilterCacheBuilder {
    pub fn from_rules(rules: Vec<CosmeticFilter>) -> Self {
        let mut self_ = Self::default();

        for rule in rules {
            self_.add_filter(rule)
        }

        self_
    }

    pub fn add_filter(&mut self, rule: CosmeticFilter) {
        if rule.has_hostname_constraint() {
            if let Some(generic_rule) = rule.hidden_generic_rule() {
                self.add_generic_filter(generic_rule);
            }
            self.store_hostname_rule(rule);
        } else {
            self.add_generic_filter(rule);
        }
    }

    /// Add a filter, assuming it has already been determined to be a generic rule
    fn add_generic_filter(&mut self, rule: CosmeticFilter) {
        let selector = match rule.plain_css_selector() {
            Some(s) => s.to_string(),
            None => {
                // Procedural cosmetic filters cannot be generic.
                // Silently ignoring this filter.
                return;
            }
        };

        if selector.starts_with('.') {
            if let Some(key) = key_from_selector(&selector) {
                assert!(key.starts_with('.'));
                let class = key[1..].to_string();
                if key == selector {
                    self.simple_class_rules.insert(class);
                } else {
                    self.complex_class_rules.insert(class, selector);
                }
            }
        } else if selector.starts_with('#') {
            if let Some(key) = key_from_selector(&selector) {
                assert!(key.starts_with('#'));
                let id = key[1..].to_string();
                if key == selector {
                    self.simple_id_rules.insert(id);
                } else {
                    self.complex_id_rules.insert(id, selector);
                }
            }
        } else {
            self.misc_generic_selectors.insert(selector);
        }
    }

    // TODO: review this
    fn store_hostname_rule(&mut self, rule: CosmeticFilter) {
        use SpecificFilterType::*;

        let unhide = rule.mask.contains(CosmeticFilterMask::UNHIDE);
        let script_inject = rule.mask.contains(CosmeticFilterMask::SCRIPT_INJECT);

        let kind = match (
            script_inject,
            rule.plain_css_selector().map(|s| s.to_string()),
            rule.action,
        ) {
            (false, Some(selector), None) => Hide(selector),
            (true, Some(selector), None) => InjectScript((selector, rule.permission)),
            (false, selector, action) => ProceduralOrAction(
                serde_json::to_string(&ProceduralOrActionFilter {
                    selector: selector
                        .map(|selector| vec![CosmeticFilterOperator::CssSelector(selector)])
                        .unwrap_or(rule.selector),
                    action,
                })
                .unwrap(),
            ),
            (true, _, Some(_)) => return, // script injection with action - shouldn't be possible
            (true, None, _) => return, // script injection without plain CSS selector - shouldn't be possible
        };

        let kind = if unhide { kind.negated() } else { kind };

        let tokens_to_insert = std::iter::empty()
            .chain(rule.hostnames.unwrap_or_default())
            .chain(rule.entities.unwrap_or_default());

        tokens_to_insert.for_each(|t| self.store_hostname_filter(&t, kind.clone()));

        let tokens_to_insert_negated = std::iter::empty()
            .chain(rule.not_hostnames.unwrap_or_default())
            .chain(rule.not_entities.unwrap_or_default());

        let negated = kind.negated();

        tokens_to_insert_negated.for_each(|t| self.store_hostname_filter(&t, negated.clone()));
    }

    fn store_hostname_filter(&mut self, token: &Hash, kind: SpecificFilterType) {
        use SpecificFilterType::*;

        match kind {
            // Handle hide and inject_script at top level for better deduplication
            Hide(s) => {
                self.hostname_hide.insert(*token, s);
            }
            InjectScript((s, permission)) => {
                let encoded_script = encode_script_with_permission(s, permission);
                self.hostname_inject_script.insert(*token, encoded_script);
            }
            // Handle remaining types through HostnameRule
            Unhide(s) => {
                let entry = self.specific_rules.entry(*token).or_default();
                entry.unhide.push(s);
            }
            UninjectScript((s, _)) => {
                let entry = self.specific_rules.entry(*token).or_default();
                entry.uninject_script.push(s);
            }
            ProceduralOrAction(s) => {
                let entry = self.specific_rules.entry(*token).or_default();
                entry.procedural_action.push(s);
            }
            ProceduralOrActionException(s) => {
                let entry = self.specific_rules.entry(*token).or_default();
                entry.procedural_action_exception.push(s);
            }
        }
    }
}

impl CosmeticFilterCache {
    pub fn from_context(filter_data_context: FilterDataContextRef) -> Self {
        Self {
            filter_data_context,
        }
    }

    #[cfg(test)]
    pub fn from_rules(rules: Vec<CosmeticFilter>) -> Self {
        use crate::filters::{
            fb_builder::make_flatbuffer_from_rules, fb_network::FilterDataContext,
        };

        let memory = make_flatbuffer_from_rules(vec![], rules, true, 0);

        let filter_data_context = FilterDataContext::new(memory);
        Self::from_context(filter_data_context)
    }

    /// Generic class/id rules are by far the most common type of cosmetic filtering rule, and they
    /// apply to all sites. Rather than injecting all of these rules onto every page, which would
    /// blow up memory usage, we only inject rules based on classes and ids that actually appear on
    /// the page (in practice, a `MutationObserver` is used to identify those elements). We can
    /// include rules like `.a-class div#ads > .advertisement`, keyed by the `.a-class` selector,
    /// since we know that this rule cannot possibly apply unless there is an `.a-class` element on
    /// the page.
    ///
    /// This method returns all of the generic CSS selectors of elements to hide (i.e. with a
    /// `display: none !important` CSS rule) that could possibly be or become relevant to the page
    /// given the new classes and ids that have appeared on the page. It guarantees that it will be
    /// safe to hide those elements on a particular page by taking into account the page's
    /// hostname-specific set of exception rules.
    ///
    /// The exceptions should be returned directly as they appear in the page's
    /// `UrlSpecificResources`. The exceptions, along with the set of already-seen classes and ids,
    /// must be cached externally as the cosmetic filtering subsystem here is designed to be
    /// stateless with regard to active page sessions.
    pub fn hidden_class_id_selectors(
        &self,
        classes: impl IntoIterator<Item = impl AsRef<str>>,
        ids: impl IntoIterator<Item = impl AsRef<str>>,
        exceptions: &HashSet<String>,
    ) -> Vec<String> {
        let mut selectors = vec![];

        let cs = self.filter_data_context.memory.root().cosmetic_filters();
        let simple_class_rules = FlatSetView::new(cs.simple_class_rules());
        let simple_id_rules = FlatSetView::new(cs.simple_id_rules());
        let complex_class_rules = FlatMapStringView::new(
            cs.complex_class_rules_index(),
            cs.complex_class_rules_values(),
        );
        let complex_id_rules =
            FlatMapStringView::new(cs.complex_id_rules_index(), cs.complex_id_rules_values());

        classes.into_iter().for_each(|class| {
            let class = class.as_ref();
            if simple_class_rules.contains(class) && !exceptions.contains(&format!(".{}", class)) {
                selectors.push(format!(".{}", class));
            }
            if let Some(bucket) = complex_class_rules.get(class) {
                for (_, sel) in bucket {
                    if !exceptions.contains(sel) {
                        selectors.push(sel.to_string());
                    }
                }
            }
        });
        ids.into_iter().for_each(|id| {
            let id = id.as_ref();
            if simple_id_rules.contains(id) && !exceptions.contains(&format!("#{}", id)) {
                selectors.push(format!("#{}", id));
            }
            if let Some(bucket) = complex_id_rules.get(id) {
                for (_, sel) in bucket {
                    if !exceptions.contains(sel) {
                        selectors.push(sel.to_string());
                    }
                }
            }
        });

        selectors
    }

    /// Any rules that can't be handled by `hidden_class_id_selectors` are returned by
    /// `hostname_cosmetic_resources`. As soon as a page navigation is committed, this method
    /// should be queried to get the initial set of cosmetic filtering operations to apply to the
    /// page. This provides any rules specifying elements to hide by selectors that are too complex
    /// to be returned by `hidden_class_id_selectors` (i.e. not directly starting with a class or
    /// id selector, like `div[class*="Ads"]`), or any rule that is only applicable to a particular
    /// hostname or set of hostnames (like `example.com##.a-class`). The first category is always
    /// injected into every page, and makes up a relatively small number of rules in practice.
    pub fn hostname_cosmetic_resources(
        &self,
        resources: &ResourceStorage,
        hostname: &str,
        generichide: bool,
    ) -> UrlSpecificResources {
        let domain_str = {
            let (start, end) = crate::url_parser::get_host_domain(hostname);
            &hostname[start..end]
        };

        let (request_entities, request_hostnames) = hostname_domain_hashes(hostname, domain_str);

        let mut specific_hide_selectors = HashSet::new();
        let mut procedural_actions = HashSet::new();
        let mut script_injections = HashMap::<&str, PermissionMask>::new();
        let mut exceptions = HashSet::new();

        let mut except_all_scripts = false;

        let hashes: Vec<&Hash> = request_entities
            .iter()
            .chain(request_hostnames.iter())
            .collect();

        let cf = self.filter_data_context.memory.root().cosmetic_filters();
        let hostname_rules_view = FlatMapView::new(cf.hostname_index(), cf.hostname_values());
        let hostname_hide_view =
            FlatMultiMapView::new(cf.hostname_hide_index(), cf.hostname_hide_values());
        let hostname_inject_script_view = FlatMultiMapView::new(
            cf.hostname_inject_script_index(),
            cf.hostname_inject_script_values(),
        );

        for hash in hashes.iter() {
            // Handle top-level hide selectors
            if let Some(hide_iterator) = hostname_hide_view.get(**hash) {
                for (_, hide_selector) in hide_iterator {
                    if !exceptions.contains(hide_selector) {
                        specific_hide_selectors.insert(hide_selector.to_owned());
                    }
                }
            }

            // Handle top-level inject scripts with encoded permissions
            if let Some(script_iterator) = hostname_inject_script_view.get(**hash) {
                for (_, encoded_script) in script_iterator {
                    let (permission, script) = decode_script_with_permission(encoded_script);
                    script_injections
                        .entry(script)
                        .and_modify(|entry| *entry |= permission)
                        .or_insert(permission);
                }
            }

            // Handle remaining rule types from HostnameSpecificRules
            if let Some(hostname_rules) = hostname_rules_view.get(**hash) {
                // Process procedural actions
                if let Some(procedural_actions_rules) = hostname_rules.procedural_action() {
                    for action in procedural_actions_rules.iter() {
                        procedural_actions.insert(action.to_owned());
                    }
                }
            }
        }

        // Process unhide/exception filters
        for hash in hashes.iter() {
            if let Some(hostname_rules) = hostname_rules_view.get(**hash) {
                // Process unhide selectors (special behavior: they also go in exceptions)
                if let Some(unhide_rules) = hostname_rules.unhide() {
                    for selector in unhide_rules.iter() {
                        specific_hide_selectors.remove(selector);
                        exceptions.insert(selector.to_owned());
                    }
                }

                // Process procedural action exceptions
                if let Some(procedural_exceptions) = hostname_rules.procedural_action_exception() {
                    for action in procedural_exceptions.iter() {
                        procedural_actions.remove(action);
                    }
                }

                // Process script uninjects
                if let Some(uninject_scripts) = hostname_rules.uninject_script() {
                    for script in uninject_scripts.iter() {
                        if script.is_empty() {
                            except_all_scripts = true;
                            script_injections.clear();
                        }
                        if except_all_scripts {
                            continue;
                        }
                        script_injections.remove(script);
                    }
                }
            }
        }

        let hide_selectors = if generichide {
            specific_hide_selectors
        } else {
            let cs = self.filter_data_context.memory.root().cosmetic_filters();
            let misc_generic_selectors_vector = cs.misc_generic_selectors();

            // TODO: check performance of this
            let mut hide_selectors = HashSet::new();
            for i in 0..misc_generic_selectors_vector.len() {
                let selector = misc_generic_selectors_vector.get(i);
                if !exceptions.contains(selector) {
                    hide_selectors.insert(selector.to_string());
                }
            }
            specific_hide_selectors.into_iter().for_each(|sel| {
                hide_selectors.insert(sel);
            });
            hide_selectors
        };

        let injected_script = resources.get_scriptlet_resources(script_injections);

        UrlSpecificResources {
            hide_selectors,
            procedural_actions,
            exceptions,
            injected_script,
            generichide,
        }
    }
}

impl<'a, B: FlatBuilder<'a>> FlatSerialize<'a, B> for CosmeticFilterCacheBuilder {
    type Output = WIPOffset<fb::CosmeticFilters<'a>>;
    fn serialize(value: Self, builder: &mut B) -> WIPOffset<fb::CosmeticFilters<'a>> {
        let complex_class_rules = FlatMultiMapBuilder::finish(value.complex_class_rules, builder);
        let complex_id_rules = FlatMultiMapBuilder::finish(value.complex_id_rules, builder);

        // Handle top-level hostname hide and inject_script for better deduplication
        let hostname_hide = FlatMultiMapBuilder::finish(value.hostname_hide, builder);
        let hostname_inject_script =
            FlatMultiMapBuilder::finish(value.hostname_inject_script, builder);

        // Handle remaining rule types through HostnameSpecificRules
        let hostname_specific_rules = FlatMapBuilder::finish(value.specific_rules, builder);

        let simple_class_rules = Some(FlatSerialize::serialize(value.simple_class_rules, builder));
        let simple_id_rules = Some(FlatSerialize::serialize(value.simple_id_rules, builder));
        let misc_generic_selectors = Some(FlatSerialize::serialize(
            value.misc_generic_selectors,
            builder,
        ));

        fb::CosmeticFilters::create(
            builder.raw_builder(),
            &fb::CosmeticFiltersArgs {
                simple_class_rules,
                simple_id_rules,
                misc_generic_selectors,
                complex_class_rules_index: Some(complex_class_rules.keys),
                complex_class_rules_values: Some(complex_class_rules.values),
                complex_id_rules_index: Some(complex_id_rules.keys),
                complex_id_rules_values: Some(complex_id_rules.values),
                hostname_hide_index: Some(hostname_hide.keys),
                hostname_hide_values: Some(hostname_hide.values),
                hostname_inject_script_index: Some(hostname_inject_script.keys),
                hostname_inject_script_values: Some(hostname_inject_script.values),
                hostname_index: Some(hostname_specific_rules.keys),
                hostname_values: Some(hostname_specific_rules.values),
            },
        )
    }
}

/// Representations of filters with complex behavior that relies on in-page JS logic.
///
/// These get stored in-memory as JSON and should be deserialized/acted on by a content script.
/// JSON is pragmatic here since there are relatively fewer of these type of rules, and they will
/// be handled by in-page JS anyways.
#[derive(Deserialize, Serialize, Clone)]
pub struct ProceduralOrActionFilter {
    /// A selector for elements that this filter applies to.
    /// This may be a plain CSS selector, or it can consist of multiple procedural operators.
    pub selector: Vec<CosmeticFilterOperator>,
    /// An action to apply to matching elements.
    /// If no action is present, the filter assumes default behavior of hiding the element with
    /// a style of `display: none !important`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<CosmeticFilterAction>,
}

impl ProceduralOrActionFilter {
    /// Returns `(selector, style)` if the filter can be expressed in pure CSS.
    pub fn as_css(&self) -> Option<(String, String)> {
        match (&self.selector[..], &self.action) {
            ([CosmeticFilterOperator::CssSelector(selector)], None) => {
                Some((selector.to_string(), "display: none !important".to_string()))
            }
            (
                [CosmeticFilterOperator::CssSelector(selector)],
                Some(CosmeticFilterAction::Style(style)),
            ) => Some((selector.to_string(), style.to_string())),
            _ => None,
        }
    }

    /// Convenience constructor for pure CSS style filters.
    #[cfg(test)]
    pub(crate) fn from_css(selector: String, style: String) -> Self {
        Self {
            selector: vec![CosmeticFilterOperator::CssSelector(selector)],
            action: Some(CosmeticFilterAction::Style(style)),
        }
    }
}

/// Exists to use common logic for binning filters correctly
#[derive(Clone)]
enum SpecificFilterType {
    Hide(String),
    Unhide(String),
    InjectScript((String, PermissionMask)),
    UninjectScript((String, PermissionMask)),
    ProceduralOrAction(String),
    ProceduralOrActionException(String),
}

impl SpecificFilterType {
    fn negated(self) -> Self {
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

fn hostname_domain_hashes(hostname: &str, domain: &str) -> (Vec<Hash>, Vec<Hash>) {
    let request_entities =
        crate::filters::cosmetic::get_entity_hashes_from_labels(hostname, domain);
    let request_hostnames =
        crate::filters::cosmetic::get_hostname_hashes_from_labels(hostname, domain);

    (request_entities, request_hostnames)
}

/// Returns the first token of a CSS selector.
///
/// This should only be called once `selector` has been verified to start with either a "#" or "."
/// character.
fn key_from_selector(selector: &str) -> Option<String> {
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

#[cfg(test)]
#[path = "../tests/unit/cosmetic_filter_cache.rs"]
mod unit_tests;
