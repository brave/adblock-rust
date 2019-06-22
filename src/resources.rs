use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Resource {
    pub content_type: String,
    pub data: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Resources {
    pub resources: HashMap<String, Resource>
}

impl Default for Resources {
    fn default() -> Resources {
        Resources {
            resources: HashMap::new()
        }
    }
}

impl Resources {
    pub fn parse(data: &str) -> Resources {
        let chunks = data.split("\n\n");
        let mut type_to_resource: HashMap<String, HashMap<String, String>> = HashMap::new();

        lazy_static! {
            static ref COMMENTS_RE: Regex = Regex::new(r"(?m:^\s*#.*$)").unwrap();
        }

        for chunk in chunks {
            let resource: String = COMMENTS_RE.replace_all(&chunk, "").to_string();
            let resource: String = resource.trim().to_owned();
            if resource.is_empty() {
                continue;
            }
            let first_new_line = resource.find("\n");
            let first_new_line_pos;
            // No new line, but appears to encode mime type and teh content is not base64, so can be empty
            if first_new_line.is_none() && resource.contains(" ") && resource.contains("/") && !resource.contains(";base64") {
                first_new_line_pos = resource.len();
            } else if first_new_line.is_none() {
                continue;
            } else {
                first_new_line_pos = first_new_line.unwrap();
            }
            let (first_line, body) = resource.split_at(first_new_line_pos);
            let mut first_line_items = first_line.split_whitespace();
            let (name, rtype) = (
                first_line_items.next(),
                first_line_items.next()
            );
            if name.is_none() || rtype.is_none() {
                continue;
            }
            let rtype = rtype.unwrap().to_owned();
            let name = name.unwrap().to_owned();
            let body = body.trim().to_owned();
            
            let ttr = type_to_resource.entry(rtype).or_insert(HashMap::new());
            ttr.insert(name, body);
        }

        // Create a mapping from resource name to { contentType, data }
        // used for request redirection.
        let mut resources: HashMap<String, Resource> = HashMap::new();
        for (content_type, type_resources) in type_to_resource {
            for (name, resource) in type_resources {
                resources.insert(name, Resource {
                    content_type: content_type.to_owned(),
                    data: resource
                });
            }
        }

        Resources {
            resources,
        }
    }

    pub fn get_resource(&self, name: &str) -> Option<&Resource> {
        self.resources.get(name)
    }

    pub fn add_resource(&mut self, name: String, resource: Resource) {
        &self.resources.insert(name, resource);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::utils;

    #[test]
    fn parses_empty_resources() {
        let resources = Resources::parse("");
        assert!(resources.resources.is_empty());
    }

    #[test]
    fn parses_one_resource() {
        let resources_str = "foo application/javascript\ncontent";
        let resources = Resources::parse(resources_str);
        assert!(resources.resources.is_empty() == false);
        let mut expected = HashMap::new();
        expected.insert("foo".to_owned(), Resource {
            content_type: "application/javascript".to_owned(),
            data: "content".to_owned()
        });
        assert_eq!(resources.resources, expected);
    }

    #[test]
    fn parses_two_resources() {
        let resources_str = r###"
foo application/javascript
content1

pixel.png image/png;base64
content2"###;
        let resources = Resources::parse(resources_str);
        assert!(resources.resources.is_empty() == false);
        let mut expected = HashMap::new();
        expected.insert("foo".to_owned(), Resource {
            content_type: "application/javascript".to_owned(),
            data: "content1".to_owned()
        });
        expected.insert("pixel.png".to_owned(), Resource {
            content_type: "image/png;base64".to_owned(),
            data: "content2".to_owned()
        });
        assert_eq!(resources.resources, expected);
    }

    #[test]
    fn robust_to_weird_format() {
        let resources_str = r###"
# Comment
    # Comment 2
foo application/javascript
content1
# Comment 3

# Type missing
pixel.png
content

# Content missing
pixel.png image/png;base64

# This one is good!
pixel.png   image/png;base64
content2
"###;

        let resources = Resources::parse(resources_str);
        assert!(resources.resources.is_empty() == false);
        let mut expected = HashMap::new();
        expected.insert("foo".to_owned(), Resource {
            content_type: "application/javascript".to_owned(),
            data: "content1".to_owned()
        });
        expected.insert("pixel.png".to_owned(), Resource {
            content_type: "image/png;base64".to_owned(),
            data: "content2".to_owned()
        });
        assert_eq!(resources.resources, expected);
    }

    #[test]
    fn parses_noop_resources() {
        let resources_str = r###"
nooptext text/plain


noopcss text/css


"###;
        let resources = Resources::parse(resources_str);
        assert!(resources.resources.is_empty() == false);
        let mut expected = HashMap::new();
        expected.insert("nooptext".to_owned(), Resource {
            content_type: "text/plain".to_owned(),
            data: "".to_owned()
        });
        expected.insert("noopcss".to_owned(), Resource {
            content_type: "text/css".to_owned(),
            data: "".to_owned()
        });
        assert_eq!(resources.resources, expected);
    }

    #[test]
    fn handles_ubo_resources() {
        let resources_lines = utils::read_file_lines("data/uBlockOrigin/resources.txt");
        let resources_str = resources_lines.join("\n");
        assert!(!resources_str.is_empty());
        let resources = Resources::parse(&resources_str);
        assert!(resources.resources.is_empty() == false);
        assert_eq!(resources.resources.len(), 110);
    }
}
