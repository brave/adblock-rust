use crate::filters::cosmetic::CosmeticFilter;
use crate::filters::cosmetic::CosmeticFilterMask;
use crate::utils::Hash;

use std::collections::{HashSet, HashMap};
use std::cell::RefCell;
use std::sync::Mutex;

use psl::Psl;

lazy_static! {
    static ref PUBLIC_SUFFIXES: psl::List = psl::List::new();
}

fn generic_rules_to_stylesheet(rules: &[CosmeticFilter]) -> String {
    if rules.is_empty() {
        "".into()
    } else {
        let mut stylesheet = String::with_capacity(100 * rules.len());
        stylesheet += &rules[0].selector;
        rules.iter()
            .skip(1)
            .for_each(|rule| {
                stylesheet += ",";
                stylesheet += &rule.selector;
            });
        stylesheet += "{display:none !important;}";

        stylesheet
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HostnameSpecificResources {
    pub stylesheet: String,
    pub exceptions: HostnameExceptions,
    pub script_injections: Vec<String>,
}

impl HostnameSpecificResources {
    pub fn empty() -> Self {
        Self {
            stylesheet: String::new(),
            exceptions: HostnameExceptions::default(),
            script_injections: vec![],
        }
    }
}

fn specific_rules_to_stylesheet(rules: &[&SpecificFilterType]) -> (String, Vec<String>) {
    if rules.is_empty() {
        ("".into(), vec![])
    } else {
        let mut script_rules = Vec::with_capacity(10);

        let mut hide_stylesheet = String::with_capacity(100 * rules.len());
        let mut styled_stylesheet = String::with_capacity(10 * rules.len());

        rules.iter()
            .for_each(|rule| {
                match rule {
                    SpecificFilterType::Hide(sel) => {
                        hide_stylesheet += sel;
                        hide_stylesheet += ",";
                    }
                    SpecificFilterType::Style(sel, style) => {
                        styled_stylesheet += sel;
                        styled_stylesheet += "{";
                        styled_stylesheet += style;
                        styled_stylesheet += "}\n";
                    }
                    SpecificFilterType::ScriptInject(sel) => {
                        script_rules.push(sel.to_owned());
                    }
                    _ => unreachable!()
                }
            });

        if let Some(_trailing_comma) = hide_stylesheet.pop() {
            hide_stylesheet += "{display:none !important;}\n";
        }

        hide_stylesheet += &styled_stylesheet;

        (hide_stylesheet, script_rules)
    }
}

pub struct CosmeticFilterCache {
    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: HostnameRuleDb,

    misc_rules: Vec<CosmeticFilter>,
    // The base stylesheet can be invalidated if a new miscellaneous rule is added. RefCell is used
    // to regenerate and cache the base stylesheet behind an immutable reference if necessary.
    // Mutex is used to ensure thread-safety in the event that multiple FFI accesses occur
    // simultaneously.
    base_stylesheet: Mutex<RefCell<Option<String>>>,
}

impl CosmeticFilterCache {
    pub fn new(rules: Vec<CosmeticFilter>) -> Self {
        let mut self_ = CosmeticFilterCache {
            simple_class_rules: HashSet::with_capacity(rules.len() / 2),
            simple_id_rules: HashSet::with_capacity(rules.len() / 2),
            complex_class_rules: HashMap::with_capacity(rules.len() / 2),
            complex_id_rules: HashMap::with_capacity(rules.len() / 2),

            specific_rules: HostnameRuleDb::new(),

            misc_rules: Vec::with_capacity(rules.len() / 30),
            base_stylesheet: Mutex::new(RefCell::new(None)),
        };

        for rule in rules {
            self_.add_filter(rule)
        }

        self_.regen_base_stylesheet();

        self_
    }

    /// Rebuilds and caches the base stylesheet if necessary.
    /// This operation can be done for free if the stylesheet has not already been invalidated.
    fn regen_base_stylesheet(&self) {
        // `expect` should not fail unless a thread holding the locked mutex panics
        let base_stylesheet = self.base_stylesheet.lock().expect("Acquire base_stylesheet mutex");
        if base_stylesheet.borrow().is_none() {
            let stylesheet = generic_rules_to_stylesheet(&self.misc_rules);
            base_stylesheet.replace(Some(stylesheet));
        }
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
            self.misc_rules.push(rule);
            // `expect` should not fail unless a thread holding the locked mutex panics
            self.base_stylesheet.lock().expect("acquire base_stylesheet mutex").replace(None);
        }
    }

    pub fn class_id_stylesheet(&self, classes: &[String], ids: &[String], exceptions: &HostnameExceptions) -> Option<String> {
        let mut simple_classes = vec![];
        let mut simple_ids = vec![];
        let mut complex_selectors = vec![];

        classes.iter().for_each(|class| {
            if self.simple_class_rules.contains(class) {
                if exceptions.allow_generic_selector(&format!(".{}", class)) {
                    simple_classes.push(class);
                }
            }
            if let Some(bucket) = self.complex_class_rules.get(class) {
                complex_selectors.extend(bucket.iter().filter(|sel| {
                    exceptions.allow_generic_selector(sel)
                }));
            }
        });
        ids.iter().for_each(|id| {
            if self.simple_id_rules.contains(id) {
                if exceptions.allow_generic_selector(&format!("#{}", id)) {
                    simple_ids.push(id);
                }
            }
            if let Some(bucket) = self.complex_id_rules.get(id) {
                complex_selectors.extend(bucket.iter().filter(|sel| {
                    exceptions.allow_generic_selector(sel)
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

    pub fn hostname_stylesheet(&self, hostname: &str) -> HostnameSpecificResources {
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

        let mut exceptions = HostnameExceptions::default();

        rules_that_apply.iter().for_each(|r| {
            exceptions.insert_if_exception(r);
        });

        let rules_that_apply = rules_that_apply.iter().map(|r| r.to_owned()).filter(|r| {
            exceptions.allow_specific_rule(r)
        }).collect::<Vec<_>>();

        let (stylesheet, script_injections) = specific_rules_to_stylesheet(&rules_that_apply[..]);

        HostnameSpecificResources {
            stylesheet,
            exceptions,
            script_injections,
        }
    }

    pub fn base_stylesheet(&self) -> String {
        self.regen_base_stylesheet();
        self.base_stylesheet
            .lock()
            // `expect` should not fail unless a thread holding the locked mutex panics
            .expect("Acquire base_stylesheet mutex")
            .borrow()
            .as_ref()
            // Unwrap is safe because the stylesheet is regenerated above if it is None
            .unwrap()
            .clone()
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct HostnameExceptions {
    hide_exceptions: HashSet<String>,
    style_exceptions: HashSet<(String, String)>,
    script_inject_exceptions: HashSet<String>,
}

impl HostnameExceptions {
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

#[derive(Clone, Debug)]
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

        let out = cfcache.hostname_stylesheet("test.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_stylesheet("example.com");
        expected.exceptions.hide_exceptions.insert(".item".into());
        assert_eq!(out, expected);

        let out = cfcache.hostname_stylesheet("sub.example.com");
        expected.exceptions.hide_exceptions.insert(".item2".into());
        assert_eq!(out, expected);
    }

    #[test]
    fn exceptions2() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##.item",
        ]);

        let out = cfcache.hostname_stylesheet("test.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_stylesheet("example.com");
        expected.stylesheet = ".item{display:none !important;}\n".into();
        assert_eq!(out, expected);

        let out = cfcache.hostname_stylesheet("sub.example.com");
        let mut expected = HostnameSpecificResources::empty();
        expected.exceptions.hide_exceptions.insert(".item".into());
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
        expected.stylesheet = ".element{display:none !important;}\n".into();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("test.example.com");
        expected.stylesheet = ".element{background: #fff}\n".into();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("a2.sub.example.com");
        expected.stylesheet = ".element{background: #000}\n".into();
        assert_eq!(out, expected);
    }

    #[test]
    fn script_exceptions() {
        let cfcache = cache_from_rules(vec![
            "example.com,~sub.example.com##+js(set-constant.js, atob, trueFunc)",
            "sub.test.example.com#@#+js(set-constant.js, atob, trueFunc)",
            "cosmetic.net##+js(nowebrtc.js)",
            "g.cosmetic.net##+js(window.open-defuser.js)",
            "c.g.cosmetic.net#@#+js(nowebrtc.js)",
        ]);

        let out = cfcache.hostname_cosmetic_resources("sub.example.com");
        let mut expected = HostnameSpecificResources::empty();
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("sub.test.example.com");
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("test.example.com");
        expected.script_injections = vec!["set-constant.js, atob, trueFunc".into()];
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("cosmetic.net");
        expected.script_injections = vec!["nowebrtc.js".into()];
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("g.cosmetic.net");
        expected.script_injections = vec![
            "nowebrtc.js".into(),
            "window.open-defuser.js".into()
        ];
        assert_eq!(out, expected);

        let out = cfcache.hostname_cosmetic_resources("c.g.cosmetic.net");
        expected.script_injections = vec!["window.open-defuser.js".into()];
        assert_eq!(out, expected);
    }

    #[test]
    fn base_stylesheet() {
        let cfcache = cache_from_rules(vec![
            "##a[href=\"https://ads.com\"]",
            "~test.com##div > p.ads",
            "example.com,~sub.example.com##[href^=\"http://malware.ru\"]",
            "###simple-generic",
            "##.complex #generic",
        ]);

        let out = cfcache.base_stylesheet();
        assert_eq!(out, "a[href=\"https://ads.com\"],div > p.ads{display:none !important;}".to_string());
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

        let out = cfcache.class_id_stylesheet(&vec!["with".into()], &vec![], &HostnameExceptions::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["with".into()], &HostnameExceptions::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["a-class".into()], &HostnameExceptions::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec!["simple-id".into()], &vec![], &HostnameExceptions::default());
        assert_eq!(out, None);

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &HostnameExceptions::default());
        assert_eq!(out, Some(".a-class,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec![], &HostnameExceptions::default());
        assert_eq!(out, Some(".a-class,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["simple-id".into()], &HostnameExceptions::default());
        assert_eq!(out, Some("#simple-id{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &HostnameExceptions::default());
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
        let exceptions = cfcache.hostname_stylesheet("example.co.uk").exceptions;

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &exceptions);
        assert_eq!(out, Some(".a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &exceptions);
        assert_eq!(out, Some("#simple-id,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["test-element".into()], &exceptions);
        assert_eq!(out, Some("#test-element{display:none !important;}".to_string()));

        let exceptions = cfcache.hostname_stylesheet("a1.test.com").exceptions;

        let out = cfcache.class_id_stylesheet(&vec!["a-class".into()], &vec![], &exceptions);
        assert_eq!(out, Some(".a-class,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec!["children".into(), "a-class".into()], &vec!["simple-id".into()], &exceptions);
        assert_eq!(out, Some(".a-class,#simple-id,.children .including #simple-id,.a-class .with .children{display:none !important;}".to_string()));

        let out = cfcache.class_id_stylesheet(&vec![], &vec!["test-element".into()], &exceptions);
        assert_eq!(out, None);
    }
}
