pub mod decl;
pub mod item;
pub mod lifetime;
pub mod name;
pub mod to_rust;
pub mod xml;

use crate::name::TypeName;
use item::Items;
use std::{collections::HashMap, ffi::OsStr, fs, path::Path};
use tracing::{debug, error_span};

/// Holds the analysis results for easy querying.
#[derive(Debug)]
pub struct AnalysisResult {
    pub items: Items,
    type_has_lifetime: HashMap<TypeName, bool>,
}

impl AnalysisResult {
    pub fn type_has_lifetime(&self, type_name: TypeName) -> bool {
        self.type_has_lifetime[&type_name]
    }
}

#[derive(Debug)]
pub struct Analysis {
    vk: Library,
    video: Library,
    result: AnalysisResult,
}

impl Analysis {
    /// Analyse the provided copy of the
    /// [Vulkan-Headers](https://github.com/KhronosGroup/Vulkan-Headers) repo.
    pub fn new(vulkan_headers_path: impl AsRef<Path>) -> Analysis {
        let vulkan_headers_path = vulkan_headers_path.as_ref();
        let vk = Library::new(vulkan_headers_path.join("registry/vk.xml"));
        let video = Library::new(vulkan_headers_path.join("registry/video.xml"));

        let items = Items::collect(&[&vk, &video]);
        Analysis {
            vk,
            video,
            result: AnalysisResult {
                type_has_lifetime: lifetime::lifetime_propagation(&items.types),
                items,
            },
        }
    }

    pub fn vk(&self) -> &Library {
        &self.vk
    }

    pub fn video(&self) -> &Library {
        &self.video
    }

    pub fn result(&self) -> &AnalysisResult {
        &self.result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryName {
    Vk,
    Video,
}

#[derive(Debug)]
pub struct Library {
    name: LibraryName,
    xml: xml::Registry,
}

impl Library {
    fn new(xml_path: impl AsRef<Path>) -> Library {
        let xml_path = xml_path.as_ref();
        let library_name = match (xml_path.file_name())
            .and_then(OsStr::to_str)
            .expect("path should point to xml")
        {
            "vk.xml" => LibraryName::Vk,
            "video.xml" => LibraryName::Video,
            other => panic!("unsupported library name {other:?}"),
        };

        let xml = error_span!("xml", path = %xml_path.display()).in_scope(|| {
            debug!("reading xml");
            // We leak the input string here for convenience, to avoid explicit lifetimes.
            let xml_input = Box::leak(fs::read_to_string(xml_path).unwrap().into_boxed_str());
            debug!("parsing xml");
            xml::Registry::parse(xml_input, library_name, "vulkan")
        });

        Library {
            name: library_name,
            xml,
        }
    }

    pub fn name(&self) -> LibraryName {
        self.name
    }

    pub fn xml(&self) -> &xml::Registry {
        &self.xml
    }
}
