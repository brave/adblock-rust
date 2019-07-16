use crate::filters::cosmetic::CosmeticFilter;
use crate::filters::cosmetic::CosmeticFilterMask;
use crate::utils::Hash;

use std::collections::{HashSet, HashMap};

fn rules_to_stylesheet(rules: Vec<CosmeticFilter>) -> String {
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

pub struct CosmeticFilterCache {
    simple_class_rules: HashSet<String>,
    simple_id_rules: HashSet<String>,
    complex_class_rules: HashMap<String, Vec<String>>,
    complex_id_rules: HashMap<String, Vec<String>>,

    specific_rules: Vec<CosmeticFilter>,

    base_stylesheet: String,
}

impl CosmeticFilterCache {
    pub fn new(rules: Vec<CosmeticFilter>) -> Self {
        let mut simple_class_rules = HashSet::with_capacity(rules.len() / 2);
        let mut simple_id_rules = HashSet::with_capacity(rules.len() / 2);
        let mut complex_class_rules: HashMap<String, Vec<String>> = HashMap::with_capacity(rules.len() / 2);
        let mut complex_id_rules: HashMap<String, Vec<String>> = HashMap::with_capacity(rules.len() / 2);

        let mut specific_rules = Vec::with_capacity(rules.len() / 2);

        let mut misc_rules = Vec::with_capacity(rules.len() / 30);


        for rule in rules {
            //TODO deal with script inject and unhide rules
            if rule.mask.contains(CosmeticFilterMask::SCRIPT_INJECT) ||
                rule.mask.contains(CosmeticFilterMask::UNHIDE) {
                continue;
            }

            if rule.has_hostname_constraint() {
                specific_rules.push(rule);
            } else {
                if rule.mask.contains(CosmeticFilterMask::IS_CLASS_SELECTOR) {
                    if let Some(key) = &rule.key {
                        let key = key.clone();
                        if rule.mask.contains(CosmeticFilterMask::IS_SIMPLE) {
                            simple_class_rules.insert(key);
                        } else {
                            if let Some(bucket) = complex_class_rules.get_mut(&key) {
                                bucket.push(rule.selector);
                            } else {
                                complex_class_rules.insert(key, vec![rule.selector]);
                            }
                        }
                    }
                } else if rule.mask.contains(CosmeticFilterMask::IS_ID_SELECTOR) {
                    if let Some(key) = &rule.key {
                        let key = key.clone();
                        if rule.mask.contains(CosmeticFilterMask::IS_SIMPLE) {
                            simple_id_rules.insert(key);
                        } else {
                            if let Some(bucket) = complex_id_rules.get_mut(&key) {
                                bucket.push(rule.selector);
                            } else {
                                complex_id_rules.insert(key, vec![rule.selector]);
                            }
                        }
                    }
                } else {
                    misc_rules.push(rule);
                }
            }
        }

        let base_stylesheet = rules_to_stylesheet(misc_rules);

        CosmeticFilterCache {
            simple_class_rules,
            simple_id_rules,
            complex_class_rules,
            complex_id_rules,

            specific_rules,

            base_stylesheet,
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

    pub fn hostname_stylesheet(&self, hostname: &str, domain: &str) -> String {
        let (request_entities, request_hostnames) = hostname_domain_hashes(hostname, domain);

        // TODO it would probably be better to use hashmaps here
        rules_to_stylesheet(self.specific_rules
            .iter()
            .filter(|rule| rule.matches(&request_entities[..], &request_hostnames[..]))
            .cloned()
            .collect::<Vec<_>>())
    }

    pub fn base_stylesheet(&self) -> String {
        self.base_stylesheet.clone()
    }
}

fn hostname_domain_hashes(hostname: &str, domain: &str) -> (Vec<Hash>, Vec<Hash>) {
    let request_entities = crate::filters::cosmetic::get_entity_hashes_from_labels(hostname, domain);
    let request_hostnames = crate::filters::cosmetic::get_hostname_hashes_from_labels(hostname, domain);

    (request_entities, request_hostnames)
}
