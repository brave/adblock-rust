
use crate::blocker::{Blocker, BlockerError, BlockerOptions, BlockerResult};
use crate::lists::parse_filters;
use crate::request::Request;
use bincode;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

pub struct Engine {
    pub blocker: Blocker
}

impl Engine {
    pub fn from_rules(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, false)
    }

    pub fn from_rules_debug(network_filters: &[String]) -> Engine {
        Self::from_rules_parametrised(&network_filters, true)
    }

    fn from_rules_parametrised(network_filters: &[String], debug: bool) -> Engine {
        let (parsed_network_filters, _) = parse_filters(&network_filters, true, false, debug);

        let blocker_options = BlockerOptions {
            debug,
            enable_optimizations: true,
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
        let gz = GzDecoder::new(serialized);
        let blocker = bincode::deserialize_from(gz)
            .or_else(|e| {
                println!("Error deserializing: {:?}", e);
                Err(BlockerError::DeserializationError)
            })?;
        self.blocker = blocker;
        Ok(())
    }

    pub fn check_network_urls(&self, url: &str, source_url: &str, request_type: &str) -> BlockerResult {
        let request = Request::from_urls(&url, &source_url, &request_type).unwrap();
        self.blocker.check(&request)
    }

    pub fn check_network_urls_with_hostnames(&self, url: &str, hostname: &str, source_hostname: &str, request_type: &str, third_party_request: Option<bool>) -> BlockerResult {
        let request = Request::from_urls_with_hostname(url, hostname, source_hostname, request_type, third_party_request);
        self.blocker.check(&request)
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
}
