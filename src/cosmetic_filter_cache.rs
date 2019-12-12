use crate::filters::cosmetic::CosmeticFilter;
use crate::filters::cosmetic::CosmeticFilterMask;
use crate::resources::{Resource, ScriptletResourceStorage};
use crate::utils::Hash;

use std::collections::{HashSet, HashMap};

use serde::{Deserialize, Serialize};
use psl::Psl;

lazy_static! {
    static ref PUBLIC_SUFFIXES: psl::List = psl::List::new();
}

/// Contains cosmetic filter information intended to be injected into a particular hostname.
///
/// `hide_selectors` is a set of any CSS selector on the page that should be hidden, i.e. styled as
/// `{ display: none !important; }`.
///
/// `style_selectors` is a map of CSS selectors on the page to respective non-hide style rules,
/// i.e. any required styles other than `display: none`.
///
/// `exceptions` is a set of any class or id CSS selectors that should not have generic rules
/// applied. In practice, these should be passed to `class_id_stylesheet` and not used otherwise.
///
/// `injected_script` is the Javascript code for any scriptlets that should be injected into the
/// page.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostnameSpecificResources {
    pub hide_selectors: HashSet<String>,
    pub style_selectors: HashMap<String, Vec<String>>,
    pub exceptions: HashSet<String>,
    pub injected_script: String,
}

impl HostnameSpecificResources {
    pub fn empty() -> Self {
        Self {
            hide_selectors: HashSet::new(),
            style_selectors: HashMap::new(),
            exceptions: HashSet::new(),
            injected_script: String::new(),
        }
    }
}

fn hostname_specific_rules(rules: &[&SpecificFilterType]) -> (HashSet<String>, HashMap<String, Vec<String>>, Vec<String>) {
    if rules.is_empty() {
        (HashSet::default(), HashMap::default(), vec![])
    } else {
        let mut script_rules = Vec::with_capacity(10);

        let mut hide_rules = HashSet::with_capacity(rules.len());
        let mut style_rules: HashMap<String, Vec<String>> = HashMap::with_capacity(rules.len());

        rules.iter()
            .for_each(|rule| {
                match rule {
                    SpecificFilterType::Hide(sel) => {
                        hide_rules.insert(sel.to_owned());
                    }
                    SpecificFilterType::Style(sel, style) => {
                        if let Some(entry) = style_rules.get_mut(sel) {
                            entry.push(style.to_owned());
                        } else {
                            style_rules.insert(sel.to_owned(), vec![style.to_owned()]);
                        }
                    }
                    SpecificFilterType::ScriptInject(sel) => {
                        script_rules.push(sel.to_owned());
                    }
                    _ => unreachable!()
                }
            });

        (hide_rules, style_rules, script_rules)
    }
}

#[derive(Deserialize, Serialize)]
pub struct CosmeticFilterCache {
    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: HostnameRuleDb,

    misc_generic_selectors: HashSet<String>,

    scriptlets: ScriptletResourceStorage,
}

impl CosmeticFilterCache {
    pub fn new(rules: Vec<CosmeticFilter>) -> Self {
        let mut self_ = CosmeticFilterCache {
            simple_class_rules: HashSet::with_capacity(rules.len() / 2),
            simple_id_rules: HashSet::with_capacity(rules.len() / 2),
            complex_class_rules: HashMap::with_capacity(rules.len() / 2),
            complex_id_rules: HashMap::with_capacity(rules.len() / 2),

            specific_rules: HostnameRuleDb::new(),

            misc_generic_selectors: HashSet::with_capacity(rules.len() / 30),

            scriptlets: Default::default(),
        };

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
            self.specific_rules.store_rule(rule);
        } else {
            self.add_generic_filter(rule);
        }
    }

    /// Add a filter, assuming it has already been determined to be a generic rule
    fn add_generic_filter(&mut self, rule: CosmeticFilter) {
        if rule.mask.contains(CosmeticFilterMask::IS_CLASS_SELECTOR) {
            if let Some(key) = &rule.key {
                let key = key.clone();
                if rule.mask.contains(CosmeticFilterMask::IS_SIMPLE) {
                    self.simple_class_rules.insert(key);
                } else {
                    if let Some(bucket) = self.complex_class_rules.get_mut(&key) {
                        bucket.push(rule.selector);
                    } else {
                        self.complex_class_rules.insert(key, vec![rule.selector]);
                    }
                }
            }
        } else if rule.mask.contains(CosmeticFilterMask::IS_ID_SELECTOR) {
            if let Some(key) = &rule.key {
                let key = key.clone();
                if rule.mask.contains(CosmeticFilterMask::IS_SIMPLE) {
                    self.simple_id_rules.insert(key);
                } else {
                    if let Some(bucket) = self.complex_id_rules.get_mut(&key) {
                        bucket.push(rule.selector);
                    } else {
                        self.complex_id_rules.insert(key, vec![rule.selector]);
                    }
                }
            }
        } else {
            self.misc_generic_selectors.insert(rule.selector);
        }
    }

    pub fn class_id_stylesheet(&self, classes: &[String], ids: &[String], exceptions: &HashSet<String>) -> Option<String> {
        let mut simple_classes = vec![];
        let mut simple_ids = vec![];
        let mut complex_selectors = vec![];

        classes.iter().for_each(|class| {
            if self.simple_class_rules.contains(class) {
                if !exceptions.contains(&format!(".{}", class)) {
                    simple_classes.push(class);
                }
            }
            if let Some(bucket) = self.complex_class_rules.get(class) {
                complex_selectors.extend(bucket.iter().filter(|sel| {
                    !exceptions.contains(*sel)
                }));
            }
        });
        ids.iter().for_each(|id| {
            if self.simple_id_rules.contains(id) {
                if !exceptions.contains(&format!("#{}", id)) {
                    simple_ids.push(id);
                }
            }
            if let Some(bucket) = self.complex_id_rules.get(id) {
                complex_selectors.extend(bucket.iter().filter(|sel| {
                    !exceptions.contains(*sel)
                }));
            }
        });

        if simple_classes.is_empty() && simple_ids.is_empty() && complex_selectors.is_empty() {
            return None;
        }

        let stylesheet = simple_classes.into_iter().map(|class| format!(".{}", class))
            .chain(simple_ids.into_iter().map(|id| format!("#{}", id)))
            .chain(complex_selectors.into_iter().cloned())
            .collect::<Vec<_>>()
            .join(",") + "{display:none !important;}";

        Some(stylesheet)
    }

    pub fn hostname_cosmetic_resources(&self, hostname: &str) -> HostnameSpecificResources {
        let domain = match PUBLIC_SUFFIXES.domain(hostname) {
            Some(domain) => domain,
            None => return HostnameSpecificResources::empty(),
        };
        let domain_str = domain.to_str();

        let (request_entities, request_hostnames) = hostname_domain_hashes(hostname, domain_str);

        let mut rules_that_apply = vec![];
        for hash in request_entities.iter().chain(request_hostnames.iter()) {
            if let Some(specific_rules) = self.specific_rules.retrieve(hash) {
                rules_that_apply.extend(specific_rules);
            }
        };

        let mut exceptions = HostnameExceptionsBuilder::default();

        rules_that_apply.iter().for_each(|r| {
            exceptions.insert_if_exception(r);
        });

        let rules_that_apply = rules_that_apply.iter().map(|r| r.to_owned()).filter(|r| {
            exceptions.allow_specific_rule(r)
        }).collect::<Vec<_>>();

        let (hostname_hide_selectors, style_selectors, script_injections) = hostname_specific_rules(&rules_that_apply[..]);

        let mut hide_selectors = self.misc_generic_selectors.difference(&exceptions.hide_exceptions).cloned().collect::<HashSet<_>>();
        hostname_hide_selectors.into_iter().for_each(|sel| { hide_selectors.insert(sel); });

        let mut injected_script = String::new();
        script_injections.iter().for_each(|s| {
            if let Ok(filled_template) = self.scriptlets.get_scriptlet(&s) {
                injected_script += &filled_template;
                injected_script += "\n";
            }
        });

        HostnameSpecificResources {
            hide_selectors,
            style_selectors,
            exceptions: exceptions.hide_exceptions,
            injected_script,
        }
    }

    pub fn use_resources(&mut self, resources: &[Resource]) {
        self.scriptlets = ScriptletResourceStorage::from_resources(resources);
    }

    pub fn add_resource(&mut self, resource: &Resource) {
        self.scriptlets.add_resource(resource).unwrap_or_else(|e| eprintln!("Failed to add resource: {:?}", e));
    }
}

/// Used internally to build hostname-specific rulesets by canceling out rules which match any
/// exceptions
#[derive(Default, Debug, PartialEq, Eq)]
struct HostnameExceptionsBuilder {
    hide_exceptions: HashSet<String>,
    style_exceptions: HashSet<(String, String)>,
    script_inject_exceptions: HashSet<String>,
}

impl HostnameExceptionsBuilder {
    /// Saves the given rule if it's an exception, or ignores it otherwise.
    pub fn insert_if_exception(&mut self, rule: &SpecificFilterType) {
        use SpecificFilterType as Rule;

        match rule {
            Rule::Hide(_) | Rule::Style(_, _) | Rule::ScriptInject(_) => (),
            Rule::Unhide(sel) => {
                self.hide_exceptions.insert(sel.clone());
            }
            Rule::UnhideStyle(sel, style) => {
                self.style_exceptions.insert((sel.clone(), style.clone()));
            }
            Rule::UnhideScriptInject(script) => {
                self.script_inject_exceptions.insert(script.clone());
            }
        }
    }

    /// A generic selector is allowed if it is not excepted by this set of exceptions.
    pub fn allow_generic_selector(&self, selector: &str) -> bool {
        !self.hide_exceptions.contains(selector)
    }

    /// Specific rules are allowed if they can be used to hide, restyle, or inject a script in the
    /// context of this set of exceptions - i.e. if the rule itself is not an exception rule and
    /// doesn't have a corresponding exception rule added previously.
    pub fn allow_specific_rule(&self, rule: &SpecificFilterType) -> bool {
        match rule {
            SpecificFilterType::Hide(sel) => !self.hide_exceptions.contains(sel),
            SpecificFilterType::Style(sel, style) => !self.style_exceptions.contains(&(sel.to_string(), style.to_string())),
            SpecificFilterType::ScriptInject(sel) => !self.script_inject_exceptions.contains(sel),
            _ => false,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct HostnameRuleDb {
    db: HashMap<Hash, Vec<SpecificFilterType>>,
}

/// Each hostname-specific filter can be pointed to by several different hostnames, and each
/// hostname can correspond to several different filters. To effectively store and access those
/// filters by hostname, all the non-hostname information for filters is stored in per-hostname
/// "buckets" within a Vec, and each bucket is identified by its index. Hostname hashes are used as
/// keys to get the indices of relevant buckets, which are in turn used to retrieve all the filters
/// that apply.
impl HostnameRuleDb {
    pub fn new() -> Self {
        HostnameRuleDb {
            db: HashMap::new(),
        }
    }

    pub fn store_rule(&mut self, rule: CosmeticFilter) {
        let kind = SpecificFilterType::from(&rule);

        if let Some(hostnames) = rule.hostnames {
            hostnames.iter().for_each(|h| {
                self.store(h, kind.clone())
            });
        }
        if let Some(entities) = rule.entities {
            entities.iter().for_each(|e| {
                self.store(e, kind.clone())
            });
        }

        let kind = kind.negated();

        if let Some(not_hostnames) = rule.not_hostnames {
            not_hostnames.iter().for_each(|h| {
                self.store(h, kind.clone())
            });
        }
        if let Some(not_entities) = rule.not_entities {
            not_entities.iter().for_each(|e| {
                self.store(e, kind.clone())
            });
        }
    }

    fn store(&mut self, hostname: &Hash, kind: SpecificFilterType) {
        if let Some(bucket) = self.db.get_mut(hostname) {
            bucket.push(kind);
        } else {
            self.db.insert(hostname.clone(), vec![kind]);
        }
    }

    pub fn retrieve<'a>(&'a self, hostname: &Hash) -> Option<&'a[SpecificFilterType]> {
        if let Some(bucket) = self.db.get(hostname) {
            Some(&bucket)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SpecificFilterType {
    // Parameter is the rule's selector
    Hide(String),
    Unhide(String),

    // Parameters are the rule's selector, and its additional style
    Style(String, String),
    UnhideStyle(String, String),          // Doesn't happen in practice

    // Parameter is the rule's injected script
    ScriptInject(String),
    UnhideScriptInject(String),           // Barely happens in practice
}

/// This implementation assumes the given rule has hostname or entity constraints, and that the
/// appropriate 'hidden' generic rule has already been applied externally if necessary.
impl From<&CosmeticFilter> for SpecificFilterType {
    fn from(rule: &CosmeticFilter) -> Self {
        let unhide = rule.mask.contains(CosmeticFilterMask::UNHIDE);

        if let Some(ref style) = rule.style {
            if unhide {
                SpecificFilterType::UnhideStyle(rule.selector.clone(), style.clone())
            } else {
                SpecificFilterType::Style(rule.selector.clone(), style.clone())
            }
        } else if rule.mask.contains(CosmeticFilterMask::SCRIPT_INJECT) {
            if unhide {
                SpecificFilterType::UnhideScriptInject(rule.selector.clone())
            } else {
                SpecificFilterType::ScriptInject(rule.selector.clone())
            }
        } else {
            if unhide {
                SpecificFilterType::Unhide(rule.selector.clone())
            } else {
                SpecificFilterType::Hide(rule.selector.clone())
            }
        }
    }
}

impl SpecificFilterType {
    pub fn negated(self) -> Self {
        match self {
            SpecificFilterType::Hide(sel) => SpecificFilterType::Unhide(sel),
            SpecificFilterType::Unhide(sel) => SpecificFilterType::Hide(sel),
            SpecificFilterType::Style(sel, style) => SpecificFilterType::UnhideStyle(sel, style),
            SpecificFilterType::UnhideStyle(sel, style) => SpecificFilterType::Style(sel, style),
            SpecificFilterType::ScriptInject(script) => SpecificFilterType::UnhideScriptInject(script),
            SpecificFilterType::UnhideScriptInject(script) => SpecificFilterType::ScriptInject(script),

        }
    }
}

fn hostname_domain_hashes(hostname: &str, domain: &str) -> (Vec<Hash>, Vec<Hash>) {
    let request_entities = crate::filters::cosmetic::get_entity_hashes_from_labels(hostname, domain);
    let request_hostnames = crate::filters::cosmetic::get_hostname_hashes_from_labels(hostname, domain);

    (request_entities, request_hostnames)
}

#[cfg(test)]
mod cosmetic_cache_tests {
    use super::*;

    fn cache_from_rules(rules: Vec<&str>) -> CosmeticFilterCache {
        let parsed_rules = rules
            .iter()
            .map(|r| CosmeticFilter::parse(r, false).unwrap())
            .collect::<Vec<_>>();

        CosmeticFilterCache::new(parsed_rules)
    }

    #[test]
    fn exceptions() {
        let cfcache = cache_from_rules(vec![
            "~example.com##.item",
            "sub.example.com#@#.item2",
        ]);

        let out = cfcache.hostname_cosmetic_resources("test.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("example.com");
        expected.exceptions.insert(".item".into());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("sub.example.com");
        expected.exceptions.insert(".item2".into());
        assert_eq!(out, expected);
    }

    #[test]
    fn exceptions2() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.item",
        ]);

        let out = cfcache.hostname_cosmetic_resources("test.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("example.com");
        expected.hide_selectors.insert(".item".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("sub.example.com");
        let mut expected = HostnameSpecificResources::empty();
        expected.exceptions.insert(".item".into());
        assert_eq!(out, expected);
    }

    #[test]
    fn style_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.element:style(background: #fff)",
            "sub.test.example.com#@#.element:style(background: #fff)",
            "a1.sub.example.com##.element",
            "a2.sub.example.com##.element:style(background: #000)",
        ]);

        let out = cfcache.hostname_cosmetic_resources("sub.example.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("sub.test.example.com");
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("a1.sub.example.com");
        expected.hide_selectors.insert(".element".to_owned());
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("test.example.com");
        expected.hide_selectors.clear();
        expected.style_selectors.insert(".element".to_owned(), vec!["background: #fff".to_owned()]);
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("a2.sub.example.com");
        expected.style_selectors.clear();
        expected.style_selectors.insert(".element".to_owned(), vec!["background: #000".to_owned()]);
        assert_eq!(out, expected);
    }

    #[test]
    fn script_exceptions() {
        use crate::resources::{ResourceType, MimeType};

        let mut cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##+js(set-constant.js, atob, trueFunc)",
            "sub.test.example.com#@#+js(set-constant.js, atob, trueFunc)",
            "cosmetic.net##+js(nowebrtc.js)",
            "g.cosmetic.net##+js(window.open-defuser.js)",
            "c.g.cosmetic.net#@#+js(nowebrtc.js)",
        ]);

        cfcache.use_resources(&[
            Resource {
                name: "set-constant.js".into(),
                aliases: vec![],
                kind: ResourceType::Template,
                content: base64::encode("set-constant.js, {{1}}, {{2}}"),
            },
            Resource {
                name: "nowebrtc.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(
                    MimeType::ApplicationJavascript,
                ),
                content: base64::encode("nowebrtc.js"),
            },
            Resource {
                name: "window.open-defuser.js".into(),
                aliases: vec![],
                kind: ResourceType::Mime(
                    MimeType::ApplicationJavascript,
                ),
                content: base64::encode("window.open-defuser.js"),
            },
        ]);

        let out = cfcache.hostname_cosmetic_resources("sub.example.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("sub.test.example.com");
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("test.example.com");
        expected.injected_script = "set-constant.js, atob, trueFunc\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("cosmetic.net");
        expected.injected_script = "nowebrtc.js\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("g.cosmetic.net");
        expected.injected_script = "nowebrtc.js\nwindow.open-defuser.js\n".to_owned();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("c.g.cosmetic.net");
        expected.injected_script = "window.open-defuser.js\n".to_owned();
        assert_eq!(out, expected);
    }

    #[test]
    fn matching_class_id_stylesheet() {
        let rules = vec![
            "##.a-class",
            "###simple-id",
            "##.a-class .with .children",
            "##.children .including #simple-id",
            "##a.a-class",
        ];
        let cfcache = CosmeticFilterCache::new(rules.iter().map(|r| CosmeticFilter::parse(r, false).unwrap()).collect::<Vec<_>>());

        let out = cfcache.class_id_stylesheet(&vec!["with".into()], &vec![], &HashSet::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["with".into()], &HashSet::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["a-class".into()], &HashSet::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec!["simple-id".into()], &vec![], &HashSet::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &HashSet::default());
        assert_eq!(out, Some(".a-class,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec![], &HashSet::default());
        assert_eq!(out, Some(".a-class,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["simple-id".into()], &HashSet::default());
        assert_eq!(out, Some("#simple-id{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &HashSet::default());
        assert_eq!(out, Some(".a-class,#simple-id,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));
    }

    #[test]
    fn class_id_exceptions() {
        let rules = vec![
            "##.a-class",
            "###simple-id",
            "##.a-class .with .children",
            "##.children .including #simple-id",
            "##a.a-class",
            "example.*#@#.a-class",
            "~test.com###test-element",
        ];
        let cfcache = CosmeticFilterCache::new(rules.iter().map(|r| CosmeticFilter::parse(r, false).unwrap()).collect::<Vec<_>>());
        let exceptions = cfcache.hostname_cosmetic_resources("example.co.uk").exceptions;

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &exceptions);
        assert_eq!(out, Some(".a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &exceptions);
        assert_eq!(out, Some("#simple-id,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["test-element".into()], &exceptions);
        assert_eq!(out, Some("#test-element{display:none !important;}".to_string()));

        let exceptions = cfcache.hostname_cosmetic_resources("a1.test.com").exceptions;

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &exceptions);
        assert_eq!(out, Some(".a-class,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &exceptions);
        assert_eq!(out, Some(".a-class,#simple-id,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["test-element".into()], &exceptions);
        assert_eq!(out, None);
    }

    #[test]
    fn misc_generic_exceptions() {
        let rules = vec![
            "##a[href=\"bad.com\"]",
            "##div > p",
            "##a[href=\"notbad.com\"]",
            "example.com#@#div > p",
            "~example.com##a[href=\"notbad.com\"]",
        ];
        let cfcache = CosmeticFilterCache::new(rules.iter().map(|r| CosmeticFilter::parse(r, false).unwrap()).collect::<Vec<_>>());

        let hide_selectors = cfcache.hostname_cosmetic_resources("test.com").hide_selectors;
        let mut expected_hides = HashSet::new();
        expected_hides.insert("a[href=\"bad.com\"]".to_owned());
        expected_hides.insert("div > p".to_owned());
        expected_hides.insert("a[href=\"notbad.com\"]".to_owned());
        assert_eq!(hide_selectors, expected_hides);

        let hide_selectors = cfcache.hostname_cosmetic_resources("example.com").hide_selectors;
        let mut expected_hides = HashSet::new();
        expected_hides.insert("a[href=\"bad.com\"]".to_owned());
        assert_eq!(hide_selectors, expected_hides);
    }
}
