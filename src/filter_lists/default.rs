use crate::lists::FilterList;

pub fn default_lists() -> Vec<FilterList> {
    [
        FilterList {
            uuid: String::from("67F880F5-7602-4042-8A3D-01481FD7437A"),
            url: String::from("https://easylist.to/easylist/easylist.txt"),
            title: String::from("EasyList"),
            langs: Vec::new(),
            support_url: String::from("https://easylist.to/"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
        FilterList {
            uuid: String::from("48010209-AD34-4DF5-A80C-3D2A7C3920C0"),
            url: String::from("https://easylist.to/easylist/easyprivacy.txt"),
            title: String::from("EasyPrivacy"),
            langs: Vec::new(),
            support_url: String::from("https://easylist.to/"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
        FilterList {
            uuid: String::from("200392E7-9A0F-40DF-86EB-6AF7E4071322"),
            url: String::from(
                "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/unbreak.txt",
            ),
            title: String::from("uBlock Unbreak"),
            langs: Vec::new(),
            support_url: String::from("https://github.com/gorhill/uBlock"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
        FilterList {
            uuid: String::from("2FBEB0BC-E2E1-4170-BAA9-05E76AAB5BA5"),
            url: String::from(
                "https://raw.githubusercontent.com/brave/adblock-lists/master/brave-unbreak.txt",
            ),
            title: String::from("Brave Unblock"),
            langs: Vec::new(),
            support_url: String::from("https://github.com/brave/adblock-lists"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
        FilterList {
            uuid: String::from("BCDF774A-7845-4121-B7EB-77EB66CEDF84"),
            url: String::from(
                "https://raw.githubusercontent.com/brave/adblock-lists/master/coin-miners.txt",
            ),
            title: String::from("NoCoin Filter List"),
            langs: Vec::new(),
            support_url: String::from("https://github.com/brave/adblock-lists"),
            component_id: String::from(""),
            base64_public_key: String::from(""),
        },
    ].to_vec()
}
