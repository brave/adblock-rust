use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::resources::{Resource, ResourceType};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RedirectResource {
    pub content_type: String,
    pub data: String
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct RedirectResourceStorage {
    pub resources: HashMap<String, RedirectResource>,
}

impl RedirectResourceStorage {
    pub fn from_resources(resources: &[Resource]) -> Self {
        let mut redirectable_resources: HashMap<String, RedirectResource> = HashMap::new();

        resources.iter().filter_map(|descriptor| {
            if let ResourceType::Mime(ref content_type) = descriptor.kind {
                let resource = RedirectResource {
                    content_type: content_type.clone().into(),
                    data: descriptor.content.to_owned(),
                };
                Some((descriptor.name.to_owned(), descriptor.aliases.to_owned(), resource))
            } else {
                None
            }
        }).for_each(|(name, res_aliases, resource)| {
            res_aliases.iter().for_each(|alias| {
                redirectable_resources.insert(alias.to_owned(), resource.clone());
            });
            redirectable_resources.insert(name, resource);
        });

        Self {
            resources: redirectable_resources,
        }
    }

    pub fn get_resource(&self, name: &str) -> Option<&RedirectResource> {
        self.resources.get(name)
    }

    pub fn add_resource(&mut self, resource: Resource) {
        if let ResourceType::Mime(content_type) = resource.kind {
            let name = resource.name.to_owned();
            let redirect_resource = RedirectResource {
                content_type: content_type.into(),
                data: resource.content.to_owned(),
            };
            resource.aliases.iter().for_each(|alias| {
                self.resources.insert(alias.to_owned(), redirect_resource.clone());
            });
            self.resources.insert(name, redirect_resource);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::MimeType;

    #[test]
    fn get_resource_by_name() {
        let mut storage = RedirectResourceStorage::default();
        storage.add_resource(Resource {
            name: "name.js".to_owned(),
            aliases: vec![],
            kind: ResourceType::Mime(MimeType::ApplicationJavascript),
            content: base64::encode("resource data"),
        });

        assert_eq!(storage.get_resource("name.js"), Some(&RedirectResource {
            content_type: "application/javascript".to_owned(),
            data: base64::encode("resource data"),
        }));
    }

    #[test]
    fn get_resource_by_alias() {
        let mut storage = RedirectResourceStorage::default();
        storage.add_resource(Resource {
            name: "name.js".to_owned(),
            aliases: vec!["alias.js".to_owned()],
            kind: ResourceType::Mime(MimeType::ApplicationJavascript),
            content: base64::encode("resource data"),
        });

        assert_eq!(storage.get_resource("alias.js"), Some(&RedirectResource {
            content_type: "application/javascript".to_owned(),
            data: base64::encode("resource data"),
        }));
    }
}
