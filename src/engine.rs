
use crate::blocker::{Blocker, BlockerError, BlockerOptions, BlockerResult};
use crate::lists::parse_filters;
use crate::request::Request;
use crate::filters::network::NetworkFilter;
use bincode;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

pub struct Engine {
    pub blocker: Blocker
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
            blocker: Blocker::new(parsed_network_filters, &blocker_options)
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, BlockerError> {
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());

        bincode::serialize_into(&mut gz, &self.blocker)
            .or_else(|_| Err(BlockerError::SerializationError))?;
        
        let compressed = gz.finish()
            .or_else(|_| Err(BlockerError::SerializationError))?;
        Ok(compressed)
    }

    pub fn deserialize(&mut self, serialized: &[u8]) -> Result<(), BlockerError> {
        let current_tags = self.blocker.tags_enabled();
        let gz = GzDecoder::new(serialized);
        let blocker = bincode::deserialize_from(gz)
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
        .map_err(|e| BlockerError::from(e))
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
        .map_err(|e| BlockerError::from(e))
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

    pub fn tags_enable<'a>(&'a mut self, tags: &[&str]) -> () {
        self.blocker.tags_enable(tags);
    }

    pub fn tags_disable<'a>(&'a mut self, tags: &[&str]) -> () {
        self.blocker.tags_disable(tags);
    }

    pub fn with_resources<'a>(&'a mut self, resources: &'a str) -> &'a mut Engine {
        self.blocker.with_resources(resources);
        self
    }
    
    pub fn tag_exists(&self, tag: &str) -> bool {
        self.blocker.tags_enabled().contains(&tag.to_owned())
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
    fn deserialization_backwards_compatible() {
        {
            let serialized: Vec<u8> = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 141, 140, 177, 13, 0, 17, 24, 133, 201, 85, 87, 220, 12, 215, 92, 119, 209, 91, 192, 32, 191, 208, 42, 88, 194, 38, 18, 149, 154, 13, 108, 160, 181, 8, 137, 80, 232, 188, 230, 229, 203, 203, 251, 16, 58, 11, 158, 29, 128, 254, 229, 115, 121, 113, 123, 175, 177, 221, 147, 65, 16, 14, 74, 73, 189, 142, 213, 39, 243, 48, 27, 119, 25, 238, 64, 154, 208, 76, 120, 0, 0, 0];

            let mut deserialized_engine = Engine::from_rules(&[]);
            deserialized_engine.deserialize(&serialized).unwrap();

            let url = "http://example.com/ad-banner.gif";
            let matched_rule = deserialized_engine.check_network_urls(url, "", "");
            assert!(matched_rule.matched, "Expected match for {}", url);
        }

        {
            let serialized: Vec<u8> = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 149, 139, 189, 13, 0, 16, 16, 133, 79, 84, 166, 48, 129, 210, 36, 38, 112, 104, 47, 97, 0, 165, 214, 8, 150, 178, 15, 9, 103, 0, 175, 121, 63, 121, 31, 192, 159, 4, 251, 210, 242, 100, 197, 221, 71, 131, 158, 40, 21, 190, 201, 183, 99, 128, 214, 71, 118, 118, 214, 203, 139, 13, 199, 193, 194, 49, 115, 0, 0, 0];
            let mut deserialized_engine = Engine::from_rules(&[]);
            deserialized_engine.tags_enable(&["abc"]);
            deserialized_engine.deserialize(&serialized).unwrap();

            let url = "http://example.com/ad-banner.gif";
            let matched_rule = deserialized_engine.check_network_urls(url, "", "");
            assert!(matched_rule.matched, "Expected match for {}", url);
        }
    }
}
