use crate::flatbuffers::containers::prefix_directory::PrefixDirectory;
use crate::flatbuffers::unsafe_tools::{VerifiedFlatbufferMemory, fb_vector_to_slice};
use crate::utils::Hash;
use std::collections::HashMap;

#[cfg(feature = "single-thread")]
pub(crate) type FilterDataContextRef = std::rc::Rc<FilterDataContext>;
#[cfg(not(feature = "single-thread"))]
pub(crate) type FilterDataContextRef = std::sync::Arc<FilterDataContext>;

// The struct is used to store the flatbuffer and supporting data
// for both network filter and cosmetic filters.
// Supposed to be stored via FilterDataContextRef to avoid copying the data.
pub(crate) struct FilterDataContext {
    pub(crate) memory: VerifiedFlatbufferMemory,
    pub(crate) unique_domains_hashes_map: HashMap<Hash, u32>,
    /// Lookup acceleration for the token maps of each network filter list, indexed by
    /// [crate::filters::fb_network_builder::NetworkFilterListId].
    pub(crate) network_list_directories: Vec<NetworkFilterListDirectories>,
    pub(crate) debug: bool,
}

/// [PrefixDirectory]s for both token maps of a single network filter list.
pub(crate) struct NetworkFilterListDirectories {
    pub(crate) filter_map: PrefixDirectory,
    pub(crate) opt_domains_map: PrefixDirectory,
}

impl FilterDataContext {
    pub(crate) fn new(memory: VerifiedFlatbufferMemory) -> FilterDataContextRef {
        // Reconstruct the unique_domains_hashes_map from the flatbuffer data
        let root = memory.root();
        let debug = root.debug();
        let mut unique_domains_hashes_map: HashMap<crate::utils::Hash, u32> = HashMap::new();
        for (index, hash) in root.unique_domains_hashes().iter().enumerate() {
            unique_domains_hashes_map.insert(hash, index as u32);
        }
        let network_list_directories = root
            .network_rules()
            .iter()
            .map(|list| NetworkFilterListDirectories {
                filter_map: PrefixDirectory::new(fb_vector_to_slice(list.filter_map_index())),
                opt_domains_map: PrefixDirectory::new(fb_vector_to_slice(
                    list.opt_domains_map_index(),
                )),
            })
            .collect();
        FilterDataContextRef::new(Self {
            memory,
            unique_domains_hashes_map,
            network_list_directories,
            debug,
        })
    }
}
