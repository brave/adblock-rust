use crate::filters::flatbuffer_generated::fb;
use crate::flatbuffers::unsafe_tools::VerifiedFlatbufferMemory;
use crate::utils::Hash;
use std::collections::HashMap;

#[cfg(feature = "single-thread")]
pub(crate) type FilterDataContextRef = std::rc::Rc<FilterDataContext>;
#[cfg(not(feature = "single-thread"))]
pub(crate) type FilterDataContextRef = std::sync::Arc<FilterDataContext>;

// The struct is used to store the flatbuffer and supporting data
// for both network filter and cosmetic filters.
// Supposed to be stored via FilterDataContextRef to avoid copying the data.
#[derive(Clone, Copy)]
pub(crate) struct ToRuleCapability {
    pub has_plain: bool,
    pub has_entity: bool,
}

impl ToRuleCapability {
    #[inline(always)]
    #[cfg(test)]
    pub const fn all() -> Self {
        Self {
            has_plain: true,
            has_entity: true,
        }
    }
}

pub(crate) struct FilterDataContext {
    pub(crate) memory: VerifiedFlatbufferMemory,
    pub(crate) unique_domains_hashes_map: HashMap<Hash, u32>,
    pub(crate) has_to_plain_rules: bool,
    pub(crate) has_to_entity_rules: bool,
}

impl FilterDataContext {
    #[inline(always)]
    pub(crate) fn to_rule_capability(&self) -> ToRuleCapability {
        ToRuleCapability {
            has_plain: self.has_to_plain_rules,
            has_entity: self.has_to_entity_rules,
        }
    }
}

fn non_empty_fb_slice(data: flatbuffers::Vector<'_, u32>) -> bool {
    !data.is_empty()
}

fn scan_to_option_flags(root: fb::Engine<'_>) -> (bool, bool) {
    let mut has_to_plain_rules = false;
    let mut has_to_entity_rules = false;

    for list in root.network_rules().iter() {
        for filter in list.filter_map_values().iter() {
            if filter.opt_to_domains().is_some_and(non_empty_fb_slice)
                || filter.opt_not_to_domains().is_some_and(non_empty_fb_slice)
            {
                has_to_plain_rules = true;
            }
            if filter.opt_to_entities().is_some_and(non_empty_fb_slice)
                || filter.opt_not_to_entities().is_some_and(non_empty_fb_slice)
            {
                has_to_entity_rules = true;
            }
            if has_to_plain_rules && has_to_entity_rules {
                return (true, true);
            }
        }
    }

    (has_to_plain_rules, has_to_entity_rules)
}

impl FilterDataContext {
    pub(crate) fn new(memory: VerifiedFlatbufferMemory) -> FilterDataContextRef {
        // Reconstruct the unique_domains_hashes_map from the flatbuffer data
        let root = memory.root();
        let mut unique_domains_hashes_map: HashMap<crate::utils::Hash, u32> = HashMap::new();
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            unique_domains_hashes_map.insert(hash, index as u32);
        }
        let (has_to_plain_rules, has_to_entity_rules) = scan_to_option_flags(root);
        FilterDataContextRef::new(Self {
            memory,
            unique_domains_hashes_map,
            has_to_plain_rules,
            has_to_entity_rules,
        })
    }
}
