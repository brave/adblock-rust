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

fn rules_to_stylesheet(rules: &[CosmeticFilter]) -> String {
    if rules.is_empty() {
        "".into()
    } else {
        let mut styled_rules = Vec::with_capacity(10);

        let mut stylesheet = String::with_capacity(100 * rules.len());
        stylesheet += &rules[0].selector;
        rules.iter()
            .skip(1)
            .for_each(|rule| {
                if rule.style.is_some() {
                    styled_rules.push(rule);
                } else {
                    stylesheet += ",";
                    stylesheet += &rule.selector;
                }
            });
        stylesheet += "{display:none !important;}\n";

        styled_rules.iter()
            .for_each(|rule| {
                stylesheet += &rule.selector;
                stylesheet += " {";
                stylesheet += rule.style.as_ref().unwrap();
                stylesheet += "}\n";
            });

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
            exceptions: HostnameExceptions::new(),
            script_injections: vec![],
        }
    }
}

pub struct CosmeticFilterCache {
    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: Vec<CosmeticFilter>,

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

            specific_rules: Vec::with_capacity(rules.len() / 2),
            //specific_scripts = HashMap<String, Vec<String>>

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
            let stylesheet = rules_to_stylesheet(&self.misc_rules);
            base_stylesheet.replace(Some(stylesheet));
        }
    }

    pub fn add_filter(&mut self, rule: CosmeticFilter) {
        //TODO deal with script inject and unhide rules
        if rule.mask.contains(CosmeticFilterMask::SCRIPT_INJECT) ||
            rule.mask.contains(CosmeticFilterMask::UNHIDE)
        {
            return;
        }

        if rule.has_hostname_constraint() {
            self.specific_rules.push(rule);
        } else {
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
    }

    pub fn class_id_stylesheet(&self, classes: &[String], ids: &[String]) -> Option<String> {
        let mut simple_classes = vec![];
        let mut simple_ids = vec![];
        let mut complex_selectors = vec![];

        classes.iter().for_each(|class| {
            if !self.simple_class_rules.contains(class) {
                return;
            }
            if let Some(bucket) = self.complex_class_rules.get(class) {
                complex_selectors.extend_from_slice(&bucket[..]);
            } else {
                simple_classes.push(class);
            }
        });
        ids.iter().for_each(|id| {
            if !self.simple_id_rules.contains(id) {
                return;
            }
            if let Some(bucket) = self.complex_id_rules.get(id) {
                complex_selectors.extend_from_slice(&bucket[..]);
            } else {
                simple_ids.push(id);
            }
        });

        if simple_classes.is_empty() && simple_ids.is_empty() && complex_selectors.is_empty() {
            return None;
        }

        let stylesheet = simple_classes.into_iter().map(|class| format!(".{}", class))
            .chain(simple_ids.into_iter().map(|id| format!("#{}", id)))
            .chain(complex_selectors.into_iter())
            .collect::<Vec<_>>()
            .join(",") + "{display:none !important;}";

        Some(stylesheet)
    }

    pub fn hostname_stylesheet(&self, hostname: &str) -> String {
        let domain = match PUBLIC_SUFFIXES.domain(hostname) {
            Some(domain) => domain,
            None => return String::new(),
        };
        let domain_str = domain.to_str();

        let (request_entities, request_hostnames) = hostname_domain_hashes(hostname, domain_str);

        // TODO it would probably be better to use hashmaps here
        rules_to_stylesheet(&self.specific_rules
            .iter()
            .filter(|rule| rule.matches(&request_entities[..], &request_hostnames[..]))
            .cloned()
            .collect::<Vec<_>>())

        // TODO Investigate using something like a HostnameBasedDB for this.
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

#[derive(Debug, PartialEq, Eq)]
pub struct HostnameExceptions {
    hide_exceptions: HashSet<String>,
    style_exceptions: HashSet<(String, String)>,
    script_inject_exceptions: HashSet<String>,
}

impl HostnameExceptions {
    pub fn new() -> Self {
        HostnameExceptions {
            hide_exceptions: HashSet::new(),
            style_exceptions: HashSet::new(),
            script_inject_exceptions: HashSet::new(),
        }
    }

    /// Saves the given rule if it's an exception, or ignores it otherwise.
    pub fn insert_if_exception(&mut self, rule: &SpecificFilterType) {
        match rule {
            SpecificFilterType::Hide(_) => (),
            SpecificFilterType::Unhide(sel) => {
                self.hide_exceptions.insert(sel.clone());
            }
            _ => (), // TODO
        }
    }

    /// Rules are allowed if the rule is not an exception rule and doesn't have a corresponding
    /// exception rule added previously.
    pub fn is_allowed(&self, rule: &SpecificFilterType) -> bool {
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
        if rule.mask.contains(CosmeticFilterMask::UNHIDE) {
            SpecificFilterType::Unhide(rule.selector.clone())
        } else if let Some(ref style) = rule.style {
            SpecificFilterType::Style(rule.selector.clone(), style.clone())
        } else if rule.mask.contains(CosmeticFilterMask::SCRIPT_INJECT) {
            SpecificFilterType::ScriptInject(rule.selector.clone())
        } else {
            SpecificFilterType::Hide(rule.selector.clone())
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
