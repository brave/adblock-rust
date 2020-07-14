
use crate::blocker::{Blocker, BlockerError, BlockerOptions, BlockerResult};
use crate::cosmetic_filter_cache::{CosmeticFilterCache, UrlSpecificResources};
use crate::lists::{parse_filters, parse_filter, ParsedFilter, FilterParseError};
use crate::request::Request;
use crate::filters::network::NetworkFilter;
use crate::filters::cosmetic::CosmeticFilter;
use crate::resources::{Resource, RedirectResource};
use std::collections::HashSet;

pub struct Engine {
    pub blocker: Blocker,
    cosmetic_cache: CosmeticFilterCache,
}

impl Default for Engine {
    /// Equivalent to `Engine::from_rules`, or `Engine::from_rules_debug` when compiled in test
    /// configuration, with an empty list of rules.
    fn default() -> Self {
        #[cfg(not(test))]
        let debug = false;

        #[cfg(test)]
        let debug = true;

        let blocker_options = BlockerOptions {
            debug,
            enable_optimizations: true,
        };

        Engine {
            blocker: Blocker::new(vec![], &blocker_options),
            cosmetic_cache: CosmeticFilterCache::new(vec![]),
        }
    }
}

impl Engine {
    /// Loads cosmetic and network rules in optimized form without debug information.
    pub fn from_rules(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, true, true, false, true)
    }

    /// Loads cosmetic and network rules in optimized form with debug information.
    pub fn from_rules_debug(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, true, true, true, true)
    }

    pub fn from_rules_parametrised(filter_rules: &[String], load_network: bool, load_cosmetic: bool, debug: bool, optimize: bool) -> Engine {
        let (parsed_network_filters, parsed_cosmetic_filters) = parse_filters(&filter_rules, load_network, load_cosmetic, debug);

        let blocker_options = BlockerOptions {
            debug,
            enable_optimizations: optimize,
        };

        Engine {
            blocker: Blocker::new(parsed_network_filters, &blocker_options),
            cosmetic_cache: CosmeticFilterCache::new(parsed_cosmetic_filters),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, BlockerError> {
        use crate::data_format::SerializeFormat;

        let serialize_format = SerializeFormat::from((&self.blocker, &self.cosmetic_cache));

        serialize_format.serialize().map_err(|e| {
            eprintln!("Error serializing: {:?}", e);
            BlockerError::SerializationError
        })
    }

    pub fn deserialize(&mut self, serialized: &[u8]) -> Result<(), BlockerError> {
        use crate::data_format::DeserializeFormat;
        let current_tags = self.blocker.tags_enabled();
        let deserialize_format = DeserializeFormat::deserialize(serialized).map_err(|e| {
            eprintln!("Error deserializing: {:?}", e);
            BlockerError::DeserializationError
        })?;
        let (blocker, cosmetic_cache) = deserialize_format.into();
        self.blocker = blocker;
        self.blocker.use_tags(&current_tags.iter().map(|s| &**s).collect::<Vec<_>>());
        self.cosmetic_cache = cosmetic_cache;
        Ok(())
    }

    pub fn check_network_urls(&self, url: &str, source_url: &str, request_type: &str) -> BlockerResult {
        Request::from_urls(&url, &source_url, &request_type)
        .map(|request| {
            self.blocker.check(&request)
        })
        .unwrap_or_else(|_e| {
            BlockerResult {
                matched: false,
                explicit_cancel: false,
                important: false,
                redirect: None,
                exception: None,
                filter: None,
                error: Some("Error parsing request".to_owned())
            }
        })
        
    }

    pub fn check_network_urls_with_hostnames(
        &self,
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party_request: Option<bool>
    ) -> BlockerResult {
        let request = Request::from_urls_with_hostname(url, hostname, source_hostname, request_type, third_party_request);
        self.blocker.check(&request)
    }

    pub fn check_network_urls_with_hostnames_subset(
        &self,
        url: &str,
        hostname: &str,
        source_hostname: &str,
        request_type: &str,
        third_party_request: Option<bool>,
        previously_matched_rule: bool,
        force_check_exceptions: bool,
    ) -> BlockerResult {
        let request = Request::from_urls_with_hostname(url, hostname, source_hostname, request_type, third_party_request);
        self.blocker.check_parameterised(&request, previously_matched_rule, force_check_exceptions)
    }

    pub fn filter_exists(&self, filter: &str) -> bool {
        let filter_parsed = NetworkFilter::parse(filter, true);
        match filter_parsed.map(|f| self.blocker.filter_exists(&f)) {
            Ok(exists) => exists,
            Err(e) => {
                eprintln!("Encountered unparseable filter when checking for filter existence: {:?}", e);
                false
            }
        }
    }

    pub fn add_filter_list(&mut self, filter_list: &str) {
        let rules = filter_list.lines().map(str::to_string).collect::<Vec<_>>();
        let (parsed_network_filters, parsed_cosmetic_filters) = parse_filters(&rules, true, true, true);

        for rule in parsed_network_filters {
            self.add_network_filter(rule);
        }

        for rule in parsed_cosmetic_filters {
            self.add_cosmetic_filter(rule);
        }
    }

    pub fn filter_add(&mut self, filter: &str) {
        let filter_parsed = parse_filter(filter, true, true, true);
        match filter_parsed {
            Ok(ParsedFilter::Network(filter)) => self.add_network_filter(filter),
            Ok(ParsedFilter::Cosmetic(filter)) => self.add_cosmetic_filter(filter),
            Err(FilterParseError::Network(e)) => eprintln!("Encountered filter error {:?} when adding network filter", e),
            Err(FilterParseError::Cosmetic(e)) => eprintln!("Encountered filter error {:?} when adding cosmetic filter", e),
            Err(FilterParseError::Unsupported) => (),
            Err(FilterParseError::Unused) => (),
            Err(FilterParseError::Empty) => (),
        }
    }

    fn add_network_filter(&mut self, filter: NetworkFilter) {
        match self.blocker.add_filter(filter) {
            Ok(_) => (),
            Err(BlockerError::BadFilterAddUnsupported) => eprintln!("Adding filters with `badfilter` option dynamically is not supported"),
            Err(BlockerError::FilterExists) => eprintln!("Filter already exists"),
            Err(e) => eprintln!("Encountered unexpected error {:?} when adding network filter", e),
        }
    }

    fn add_cosmetic_filter(&mut self, filter: CosmeticFilter) {
        self.cosmetic_cache.add_filter(filter);
    }

    pub fn with_tags(&mut self, tags: &[&str]) {
        self.blocker.use_tags(tags);
    }

    pub fn tags_enable(&mut self, tags: &[&str]) {
        self.blocker.enable_tags(tags);
    }

    pub fn tags_disable(&mut self, tags: &[&str]) {
        self.blocker.disable_tags(tags);
    }
    
    pub fn tag_exists(&self, tag: &str) -> bool {
        self.blocker.tags_enabled().contains(&tag.to_owned())
    }

    pub fn with_resources(&mut self, resources: &[Resource]) {
        self.blocker.use_resources(resources);
        self.cosmetic_cache.use_resources(resources);
    }

    pub fn resource_add(&mut self, resource: Resource) {
        self.blocker.add_resource(&resource);
        self.cosmetic_cache.add_resource(&resource);
    }

    pub fn resource_get(&self, key: &str) -> Option<RedirectResource> {
        self.blocker.get_resource(key).cloned()
    }

    // Cosmetic filter functionality

    /// If any of the provided CSS classes or ids could cause a certain generic CSS hide rule
    /// (i.e. `{ display: none !important; }`) to be required, this method will return a list of
    /// CSS selectors corresponding to rules referencing those classes or ids, provided that the
    /// corresponding rules are not excepted.
    ///
    /// `exceptions` should be passed directly from `HostnameSpecificResources`.
    pub fn hidden_class_id_selectors(&self, classes: &[String], ids: &[String], exceptions: &HashSet<String>) -> Vec<String> {
        self.cosmetic_cache.hidden_class_id_selectors(classes, ids, exceptions)
    }

    /// Returns a set of cosmetic filter resources required for a particular url. Once this has
    /// been called, all CSS ids and classes on a page should be passed to
    /// `hidden_class_id_selectors` to obtain any stylesheets consisting of generic rules (if the
    /// returned `generichide` value is false).
    pub fn url_cosmetic_resources(&self, url: &str) -> UrlSpecificResources {
        let request = Request::from_url(url);
        if request.is_err() {
            return UrlSpecificResources::empty();
        }
        let request = request.unwrap();

        let generichide = self.blocker.check_generic_hide(&request);
        self.cosmetic_cache.hostname_cosmetic_resources(&request.hostname, generichide)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{ResourceType, MimeType};
    
    #[test]
    fn tags_enable_adds_tags() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            ("http://example.com/advert.html", true),
            ("http://example.com/somelongpath/test/2.html", true),
            ("https://brianbondy.com/about", true),
            ("https://brave.com/about", true),
        ];

        let mut engine = Engine::from_rules(&filters);
        engine.tags_enable(&["stuff"]);
        engine.tags_enable(&["brian"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let matched_rule = engine.check_network_urls(&url, "", "");
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn tags_disable_works() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            ("http://example.com/advert.html", false),
            ("http://example.com/somelongpath/test/2.html", false),
            ("https://brianbondy.com/about", true),
            ("https://brave.com/about", true),
        ];
        
        let mut engine = Engine::from_rules(&filters);
        engine.tags_enable(&["brian", "stuff"]);
        engine.tags_disable(&["stuff"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let matched_rule = engine.check_network_urls(&url, "", "");
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn exception_tags_inactive_by_default() {
        let filters = vec![
            String::from("adv"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("@@||brianbondy.com/$tag=brian"),
        ];
        let url_results = vec![
            ("http://example.com/advert.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brianbondy.com/advert", true),
        ];
        
        let engine = Engine::from_rules(&filters);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let matched_rule = engine.check_network_urls(&url, "", "");
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn exception_tags_works() {
        let filters = vec![
            String::from("adv"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("@@||brianbondy.com/$tag=brian"),
        ];
        let url_results = vec![
            ("http://example.com/advert.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brianbondy.com/advert", false),
        ];
        
        let mut engine = Engine::from_rules(&filters);
        engine.tags_enable(&["brian", "stuff"]);

        url_results.into_iter().for_each(|(url, expected_result)| {
            let matched_rule = engine.check_network_urls(&url, "", "");
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn serialization_retains_tags() {
        let filters = vec![
            String::from("adv$tag=stuff"),
            String::from("somelongpath/test$tag=stuff"),
            String::from("||brianbondy.com/$tag=brian"),
            String::from("||brave.com$tag=brian"),
        ];
        let url_results = vec![
            ("http://example.com/advert.html", true),
            ("http://example.com/somelongpath/test/2.html", true),
            ("https://brianbondy.com/about", false),
            ("https://brave.com/about", false),
        ];

        let mut engine = Engine::from_rules(&filters);
        engine.tags_enable(&["stuff"]);
        engine.tags_enable(&["brian"]);
        let serialized = engine.serialize().unwrap();
        let mut deserialized_engine = Engine::default();
        deserialized_engine.tags_enable(&["stuff"]);
        deserialized_engine.deserialize(&serialized).unwrap();

        url_results.into_iter().for_each(|(url, expected_result)| {
            let matched_rule = deserialized_engine.check_network_urls(&url, "", "");
            if expected_result {
                assert!(matched_rule.matched, "Expected match for {}", url);
            } else {
                assert!(!matched_rule.matched, "Expected no match for {}, matched with {:?}", url, matched_rule.filter);
            }
        });
    }

    #[test]
    fn deserialization_backwards_compatible_plain() {
        // deserialization_generate_simple();
        // assert!(false);
        let serialized: Vec<u8> = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 1, 68, 0, 187, 255, 155, 145, 128, 145, 128,
            145, 128, 145, 128, 145, 128, 145, 129, 207, 202, 167, 36, 217, 43, 56, 97, 176, 145, 158, 145, 206, 0, 3,
            31, 255, 146, 1, 145, 169, 97, 100, 45, 98, 97, 110, 110, 101, 114, 192, 192, 192, 192, 192, 192, 192, 192,
            207, 186, 136, 69, 13, 115, 187, 170, 226, 192, 192, 192, 144, 194, 195, 194, 195, 207, 77, 26, 78, 68, 0,
            0, 0];

        let mut deserialized_engine = Engine::default();
        deserialized_engine.deserialize(&serialized).unwrap();

        let url = "http://example.com/ad-banner.gif";
        let matched_rule = deserialized_engine.check_network_urls(url, "", "");
        assert!(matched_rule.matched, "Expected match for {}", url);
    }

    #[test]
    fn deserialization_backwards_compatible_tags() {
        // deserialization_generate_tags();
        // assert!(false);
        let serialized: Vec<u8> = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 149, 139, 49, 14, 64, 48, 24, 70, 137, 131, 88,
            108, 98, 148, 184, 135, 19, 252, 197, 218, 132, 3, 8, 139, 85, 126, 171, 132, 193, 32, 54, 71, 104, 218, 205,
            160, 139, 197, 105, 218, 166, 233, 5, 250, 125, 219, 203, 123, 43, 14, 238, 163, 124, 206, 228, 79, 11, 184,
            113, 195, 55, 136, 98, 181, 132, 120, 65, 157, 17, 160, 180, 233, 152, 221, 1, 164, 98, 178, 255, 242, 178,
            221, 231, 201, 0, 19, 122, 216, 92, 112, 161, 1, 58, 213, 199, 143, 114, 0, 0, 0];
        let mut deserialized_engine = Engine::default();
        
        deserialized_engine.tags_enable(&[]);
        deserialized_engine.deserialize(&serialized).unwrap();
        let url = "http://example.com/ad-banner.gif";
        let matched_rule = deserialized_engine.check_network_urls(url, "", "");
        assert!(!matched_rule.matched, "Expected NO match for {}", url);

        deserialized_engine.tags_enable(&["abc"]);
        deserialized_engine.deserialize(&serialized).unwrap();

        let url = "http://example.com/ad-banner.gif";
        let matched_rule = deserialized_engine.check_network_urls(url, "", "");
        assert!(matched_rule.matched, "Expected match for {}", url);
    }

    #[test]
    fn deserialization_backwards_compatible_resources() {
        // deserialization_generate_resources();
        // assert!(false);
        let serialized: Vec<u8> = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 61, 139, 189, 10, 64, 80, 28, 197, 201, 46,
            229, 1, 44, 54, 201, 234, 117, 174, 143, 65, 233, 18, 6, 35, 118, 229, 127, 103, 201, 230, 99, 146, 39,
            184, 177, 25, 152, 61, 13, 238, 29, 156, 83, 167, 211, 175, 115, 90, 40, 184, 203, 235, 24, 244, 219, 176,
            209, 2, 29, 156, 130, 164, 61, 68, 132, 9, 121, 166, 131, 48, 246, 19, 74, 71, 28, 69, 113, 230, 231, 25,
            101, 186, 42, 121, 86, 73, 189, 42, 95, 103, 255, 102, 219, 183, 29, 170, 127, 68, 102, 150, 86, 28, 162,
            0, 247, 3, 163, 110, 154, 146, 145, 195, 175, 245, 47, 101, 250, 113, 201, 119, 0, 0, 0];

        let mut deserialized_engine = Engine::default();
        deserialized_engine.deserialize(&serialized).unwrap();

        let url = "http://example.com/ad-banner.gif";
        let matched_rule = deserialized_engine.check_network_urls(url, "", "");
        assert!(matched_rule.matched, "Expected match for {}", url);
        assert_eq!(matched_rule.redirect, Some("data:text/plain;base64,".to_owned()), "Expected redirect to contain resource");
    }

    fn deserialization_generate_simple() {
        let engine = Engine::from_rules(&[
            "ad-banner".to_owned()
        ]);
        let serialized = engine.serialize().unwrap();
        println!("Engine serialized: {:?}", serialized);
    }

    fn deserialization_generate_tags() {
        let mut engine = Engine::from_rules(&[
            "ad-banner$tag=abc".to_owned()
        ]);
        engine.with_tags(&["abc"]);
        let serialized = engine.serialize().unwrap();
        println!("Engine serialized: {:?}", serialized);
    }

    fn deserialization_generate_resources() {
        let mut engine = Engine::from_rules(&[
            "ad-banner$redirect=nooptext".to_owned()
        ]);

        let resources = vec![
            Resource {
                name: "nooptext".to_string(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::TextPlain),
                content: base64::encode(""),
            },
            Resource {
                name: "noopcss".to_string(),
                aliases: vec![],
                kind: ResourceType::Mime(MimeType::TextPlain),
                content: base64::encode(""),
            },
        ];
        engine.with_resources(&resources);
        
        let serialized = engine.serialize().unwrap();
        println!("Engine serialized: {:?}", serialized);
    }

    #[test]
    fn redirect_resource_insertion_works() {
        let mut engine = Engine::from_rules(&[
            "ad-banner$redirect=nooptext".to_owned()
        ]);

        engine.resource_add(Resource {
            name: "nooptext".to_owned(),
            aliases: vec![],
            kind: ResourceType::Mime(MimeType::TextPlain),
            content: "".to_owned(),
        });

        let url = "http://example.com/ad-banner.gif";
        let matched_rule = engine.check_network_urls(url, "", "");
        assert!(matched_rule.matched, "Expected match for {}", url);
        assert_eq!(matched_rule.redirect, Some("data:text/plain;base64,".to_owned()), "Expected redirect to contain resource");
    }

    #[test]
    fn redirect_resource_lookup_works() {
        let script = r#"
(function() {
	;
})();

        "#;

        let mut engine = Engine::default();

        engine.resource_add(Resource {
            name: "noopjs".to_owned(),
            aliases: vec![],
            kind: ResourceType::Mime(MimeType::ApplicationJavascript),
            content: script.to_owned(),
        });
        let inserted_resource = engine.resource_get("noopjs");
        assert!(inserted_resource.is_some());
        let resource = inserted_resource.unwrap();
        assert_eq!(resource.content_type, "application/javascript");
        assert_eq!(&resource.data, script);
    }

    #[test]
    fn generichide() {
        let filters = vec![
            String::from("##.donotblock"),
            String::from("##a[href=\"generic.com\"]"),

            String::from("@@||example.com$generichide"),
            String::from("example.com##.block"),

            String::from("@@||example2.com/test.html$generichide"),
            String::from("example2.com##.block"),
        ];
        let url_results = vec![
            ("https://example.com", vec![".block"], true),
            ("https://example.com/test.html", vec![".block"], true),
            ("https://example2.com", vec![".block", "a[href=\"generic.com\"]"], false),
            ("https://example2.com/test.html", vec![".block"], true),
        ];

        let engine = Engine::from_rules(&filters);

        url_results.into_iter().for_each(|(url, expected_result, expected_generichide)| {
            let result = engine.url_cosmetic_resources(url);
            assert_eq!(result.hide_selectors, expected_result.iter().map(|s| s.to_string()).collect::<HashSet<_>>());
            assert_eq!(result.generichide, expected_generichide);
        });
    }
}
