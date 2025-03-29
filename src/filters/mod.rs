//! Contains representations and standalone behaviors of individual filter rules.

mod abstract_network;
mod network_matchers;

pub mod cosmetic;
pub mod network;

#[cfg(feature = "flatbuffers-storage")]
pub mod fb_network;
