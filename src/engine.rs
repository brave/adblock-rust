
use crate::blocker::{Blocker, BlockerError, BlockerOptions, BlockerResult};
use crate::lists::parse_filters;
use crate::request::Request;
use bincode::{serialize, deserialize};

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
            debug: debug,
            enable_optimizations: true,
            load_cosmetic_filters: false,
            load_network_filters: true
        };

        Engine {
            blocker: Blocker::new(parsed_network_filters, &blocker_options)
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, BlockerError> {
        serialize(&self.blocker)
            .or_else(|_| Err(BlockerError::SerializationError))
    }

    pub fn deserialize(&mut self, serialized: &[u8]) -> Result<(), BlockerError> {
        let blocker = deserialize(&serialized[..])
            .or_else(|_| Err(BlockerError::DeserializationError))?;
        self.blocker = blocker;
        Ok(())
    }

    pub fn check_network_urls(&self, url: &str, source_url: &str, request_type: &str) -> BlockerResult {
        let request = Request::from_urls(&url, &source_url, &request_type).unwrap();
        self.blocker.check(&request)
    }
}
