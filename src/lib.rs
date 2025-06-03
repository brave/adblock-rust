//! `adblock-rust` is the engine powering Brave's native adblocker, available as a library for
//! anyone to use. It features:
//!
//! - Network blocking
//! - Cosmetic filtering
//! - Resource replacements
//! - Hosts syntax
//! - uBlock Origin syntax extensions
//! - iOS content-blocking syntax conversion
//! - Compiling to native code or WASM
//! - Rust bindings ([crates](https://crates.io/crates/adblock))
//! - JS bindings ([npm](https://npmjs.com/adblock-rs))
//! - Community-maintained Python bindings ([pypi](https://pypi.org/project/adblock/))
//! - High performance!
//!
//! Check the [`Engine`] documentation to get started with adblocking.

#[cfg(feature = "content-blocking")]
pub mod content_blocking;

// Own modules, currently everything is exposed, will need to limit
pub mod blocker;
pub mod cosmetic_filter_cache;
pub mod data_format;
pub mod engine;
pub mod filters;
pub mod lists;
pub mod network_filter_list;
pub mod optimizer;
pub mod regex_manager;
pub mod request;
pub mod resources;
pub mod url_parser;
pub mod utils;

// Export the capnp schema for FFI consumers
pub use network_filter_list::network_filter_capnp;

#[doc(inline)]
pub use engine::Engine;
#[doc(inline)]
pub use lists::FilterSet;

#[cfg(test)]
#[path = "../tests/test_utils.rs"]
mod test_utils;

#[cfg(test)]
mod sync_tests {
    #[allow(unused)]
    fn static_assert_sync<S: Sync>() {
        let _ = core::marker::PhantomData::<S>::default();
    }

    #[test]
    #[cfg(not(feature = "unsync-regex-caching"))]
    fn assert_engine_sync() {
        static_assert_sync::<crate::engine::Engine>();
    }
}
