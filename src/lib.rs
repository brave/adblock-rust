#![allow(dead_code)]

// Own modules, currently everything is exposed, will need to limit
pub mod blocker;
#[cfg(feature = "content-blocking")]
pub mod content_blocking;
pub mod cosmetic_filter_cache;
mod data_format;
pub mod engine;
pub mod filters;
pub mod lists;
pub mod optimizer;
pub mod regex_manager;
pub mod request;
pub mod resources;
pub mod url_parser;
#[doc(hidden)]
pub mod utils;

#[cfg(test)]
mod sync_tests {
    #[allow(unused)]
    fn static_assert_sync<S: Sync>() {
        let _ = core::marker::PhantomData::<S>::default();
    }

    #[test]
    #[cfg(not(any(feature = "object-pooling", feature = "unsync-regex-caching")))]
    fn assert_engine_sync() {
        static_assert_sync::<crate::engine::Engine>();
    }
}
