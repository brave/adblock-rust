//! Contains data types used to describe remote lists of filters.

use crate::lists::FilterFormat;
use serde::{Serialize, Deserialize};

pub mod default;
pub mod regions;

/// Describes an online source of adblock rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFilterSource {
    pub uuid: String,
    pub url: String,
    pub title: String,
    pub format: FilterFormat,
    pub langs: Vec<String>,
    pub support_url: String,
    pub component_id: String,
    pub base64_public_key: String,
    pub desc: String,
}

#[cfg(test)]
pub async fn get_all_filters() -> Vec<String> {
    use futures::FutureExt;
    let default_lists = default::default_lists();

    let filters_fut: Vec<_> = default_lists
        .iter()
        .map(|list| {
            reqwest::get(&list.url)
                .then(|resp| resp
                    .expect("Could not request rules")
                    .text()
                ).map(|text| text
                    .expect("Could not get rules as text")
                    .lines()
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>()
                )
        })
        .collect();

    futures::future::join_all(filters_fut)
        .await
        .iter()
        .flatten()
        .cloned()
        .collect()
}
