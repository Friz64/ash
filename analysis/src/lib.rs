pub mod decl;
pub mod item;
pub mod lifetime_propagation;
pub mod name;
pub mod rust;
pub mod xml;

use crate::name::{ExtensionName, TypeName};
use item::Items;
use std::{collections::HashMap, ffi::OsStr, fs, path::Path};
use tracing::{debug, error_span};

/// Holds the analysis results for easy querying.
#[derive(Debug)]
pub struct AnalysisResult {
    pub items: &'static Items,
    type_has_lifetime: HashMap<TypeName, bool>,
    is_extension_provisional: HashMap<ExtensionName, bool>,
}

impl AnalysisResult {
    pub fn type_has_lifetime(&self, type_name: TypeName) -> bool {
        self.type_has_lifetime[&type_name]
    }

    pub fn is_extension_provisional(&self, ext_name: ExtensionName) -> bool {
        self.is_extension_provisional[&ext_name]
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
        let libraries = &[&vk, &video];

        let items = Box::leak(Box::new(Items::collect(libraries)));
        let is_extension_provisional = (libraries.iter())
            .flat_map(|lib| &lib.xml.extensions)
            .map(|extension| (extension.name, extension.provisional))
            .collect();

        Analysis {
            vk,
            video,
            result: AnalysisResult {
                type_has_lifetime: lifetime_propagation::run(&items.types),
                is_extension_provisional,
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
