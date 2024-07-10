pub mod cdecl;
pub mod items;
pub mod xml;

use items::{Items, RequiredBy};
use std::{collections::HashMap, fs, path::Path};
use tracing::{debug, error_span};

/// Holds the analysis results for easy querying.
#[derive(Debug)]
pub struct Analysis {
    vk: Library,
    video: Library,
    items: Items,
}

impl Analysis {
    /// Analyse the provided copy of the
    /// [Vulkan-Headers](https://github.com/KhronosGroup/Vulkan-Headers) repo.
    pub fn new(vulkan_headers_path: impl AsRef<Path>) -> Analysis {
        let vulkan_headers_path = vulkan_headers_path.as_ref();
        let vk = Library::new(vulkan_headers_path.join("registry/vk.xml"));
        let video = Library::new(vulkan_headers_path.join("registry/video.xml"));

        let mut items = Items::default();
        vk.collect_into(&mut items);
        video.collect_into(&mut items);

        Analysis { vk, video, items }
    }

    pub fn vk_xml(&self) -> &xml::Registry {
        &self.vk.xml
    }

    pub fn video_xml(&self) -> &xml::Registry {
        &self.video.xml
    }

    pub fn items(&self) -> &Items {
        &self.items
    }
}

#[derive(Debug)]
struct Library {
    xml: xml::Registry,
}

impl Library {
    fn new(xml_path: impl AsRef<Path>) -> Library {
        let xml = error_span!("xml", path = %xml_path.as_ref().display()).in_scope(|| {
            debug!("reading xml");
            // We leak the input string here for convenience, to avoid explicit lifetimes.
            let xml_input = Box::leak(fs::read_to_string(xml_path).unwrap().into_boxed_str());
            debug!("parsing xml");
            xml::Registry::parse(xml_input, "vulkan")
        });

        Library { xml }
    }

    fn collect_into(&self, items: &mut Items) {
        let mut types_require_map = HashMap::new();

        for feature in &self.xml.features {
            let required_by = RequiredBy::Feature {
                major: feature.version.major,
                minor: feature.version.minor,
            };

            for require in &feature.requires {
                for require_type in &require.types {
                    types_require_map.insert(require_type.name, required_by);
                }
            }
        }

        for extension in &self.xml.extensions {
            let required_by = RequiredBy::Extension {
                name: extension.name,
            };

            for require in &extension.requires {
                for require_type in &require.types {
                    types_require_map.insert(require_type.name, required_by);
                }
            }
        }

        items.collect(self, types_require_map);
    }
}
