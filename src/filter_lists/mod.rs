pub mod default;
pub mod regions;

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
