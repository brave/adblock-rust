use crate::lists::FilterList;

pub fn slim_list() -> Vec<FilterList> {
    [
        FilterList {
            uuid: String::from("c8200634-c017-4f33-a0bb-5d6bd4854333"),
            url: String::from("https://adblock-data.s3.amazonaws.com/ios/latest.txt"),
            title: String::from("Brave SlimList"),
            langs: Vec::new(),
            support_url: String::from("https://brave.com/"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
    ].to_vec()
}
