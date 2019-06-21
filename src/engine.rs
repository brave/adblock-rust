
use crate::blocker::{Blocker, BlockerError, BlockerOptions, BlockerResult};
use crate::lists::parse_filters;
use crate::request::Request;
use crate::filters::network::NetworkFilter;
use crate::resources::{Resources, Resource};
use rmps;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

pub struct Engine {
    pub blocker: Blocker,
}

impl Engine {
    pub fn from_rules(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, false, true)
    }

    pub fn from_rules_debug(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, true, true)
    }

    pub fn from_rules_parametrised(network_filters: &[String], debug: bool, optimize: bool) -> Engine {
        let (parsed_network_filters, _) = parse_filters(&network_filters, true, false, debug);

        let blocker_options = BlockerOptions {
            debug,
            enable_optimizations: optimize,
            load_cosmetic_filters: false,
            load_network_filters: true
        };

        Engine {
            blocker: Blocker::new(parsed_network_filters, &blocker_options),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, BlockerError> {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        rmps::encode::write(&mut gz, &self.blocker)
            .or_else(|e| {
                eprintln!("Error serializing: {:?}", e);
                Err(BlockerError::SerializationError)
            })?;

        let compressed = gz.finish()
            .or_else(|_| Err(BlockerError::SerializationError))?;
        Ok(compressed)
    }

    pub fn deserialize(&mut self, serialized: &[u8]) -> Result<(), BlockerError> {
        let current_tags = self.blocker.tags_enabled();
        let gz = GzDecoder::new(serialized);
        let blocker = rmps::decode::from_read(gz)
            .or_else(|e| {
                eprintln!("Error deserializing: {:?}", e);
                Err(BlockerError::DeserializationError)
            })?;
        self.blocker = blocker;
        self.blocker.with_tags(&current_tags.iter().map(|s| &**s).collect::<Vec<_>>());
        Ok(())
    }

    pub fn check_network_urls(&self, url: &str, source_url: &str, request_type: &str) -> BlockerResult {
        Request::from_urls(&url, &source_url, &request_type)
        .map(|request| {
            self.blocker.check(&request)
        })
        .unwrap_or_else(|_e| {
            eprintln!("Error parsing request, returning no match");
            BlockerResult {
                matched: false,
                explicit_cancel: false,
                redirect: None,
                exception: None,
                filter: None,
            }
        })
        
    }

    pub fn check_network_urls_with_hostnames(&self, url: &str, hostname: &str, source_hostname: &str, request_type: &str, third_party_request: Option<bool>) -> BlockerResult {
        let request = Request::from_urls_with_hostname(url, hostname, source_hostname, request_type, third_party_request);
        self.blocker.check(&request)
    }

    pub fn filter_exists(&self, filter: &str) -> bool {
        let filter_parsed = NetworkFilter::parse(filter, true);
        match filter_parsed
        .map_err(BlockerError::from)
        .and_then(|f| self.blocker.filter_exists(&f)) {
            Ok(exists) => exists,
            Err(e) => {
                match e {
                    BlockerError::BlockerFilterError(e) => eprintln!("Encountered filter error {:?} when checking for filter existence", e),
                    BlockerError::OptimizedFilterExistence => eprintln!("Checking for filter existence in optimized engine will not return expected results"),
                    e => eprintln!("Encountered unexpected error {:?} when checking for filter existence", e),
                }
                
                false
            }
        }
    }

    pub fn filter_add<'a>(&'a mut self, filter: &str) -> &'a mut Engine {
        let filter_parsed = NetworkFilter::parse(filter, true);
        match filter_parsed
        .map_err(BlockerError::from)
        .and_then(|f| self.blocker.filter_add(f)) {
            Ok(_b) => self,
            Err(e) => {
                match e {
                    BlockerError::BlockerFilterError(e) => eprintln!("Encountered filter error {:?} when adding", e),
                    BlockerError::BadFilterAddUnsupported => eprintln!("Adding filters with `badfilter` option dynamically is not supported"),
                    BlockerError::FilterExists => eprintln!("Filter already exists"),
                    e => eprintln!("Encountered unexpected error {:?} when checking for filter existence", e),
                }
                
                self
            }
        }
    }

    pub fn with_tags<'a>(&'a mut self, tags: &[&str]) -> &'a mut Engine {
        self.blocker.with_tags(tags);
        self
    }

    pub fn tags_enable<'a>(&'a mut self, tags: &[&str]) {
        self.blocker.tags_enable(tags);
    }

    pub fn tags_disable<'a>(&'a mut self, tags: &[&str]) {
        self.blocker.tags_disable(tags);
    }
    
    pub fn tag_exists(&self, tag: &str) -> bool {
        self.blocker.tags_enabled().contains(&tag.to_owned())
    }

    pub fn with_resources<'a>(&'a mut self, resources: &'a str) -> &'a mut Engine {
        let resources = Resources::parse(resources);
        self.blocker.with_resources(resources);
        self
    }

    pub fn resource_add<'a>(&'a mut self, key: &str, content_type: &str, data: &str) -> &'a mut Engine {
        let resource = Resource {
            content_type: content_type.to_owned(),
            data: data.to_owned()
        };
        self.blocker.resource_add(key.to_owned(), resource);
        self
    }

    pub fn resource_get(&self, key: &str) -> Option<Resource> {
        self.blocker.resource_get(key).cloned()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
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
        let mut deserialized_engine = Engine::from_rules(&[]);
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

        let mut deserialized_engine = Engine::from_rules(&[]);
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
        let mut deserialized_engine = Engine::from_rules(&[]);
        
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

        let mut deserialized_engine = Engine::from_rules(&[]);
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
        let resources_str = r###"
nooptext text/plain


noopcss text/css


"###;
        engine.with_resources(&resources_str);
        
        let serialized = engine.serialize().unwrap();
        println!("Engine serialized: {:?}", serialized);
    }

    #[test]
    fn redirect_resource_insertion_works() {
        let mut engine = Engine::from_rules(&[
            "ad-banner$redirect=nooptext".to_owned()
        ]);

        engine.resource_add("nooptext", "text/plain", "");

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

        let mut engine = Engine::from_rules(&[]);

        engine.resource_add("noopjs", "application/javascript", script);
        let inserted_resource = engine.resource_get("noopjs");
        assert!(inserted_resource.is_some());
        let resource = inserted_resource.unwrap();
        assert_eq!(resource.content_type, "application/javascript");
        assert_eq!(&resource.data, script);
    }
}
