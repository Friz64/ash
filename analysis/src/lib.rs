mod cdecl;
mod xml;

use std::{fs, path::Path};
use tracing::{debug, error_span};

#[derive(Debug)]
pub struct Analysis {
    pub vk: Library,
    pub video: Library,
}

impl Analysis {
    pub fn new(vulkan_headers_path: impl AsRef<Path>) -> Analysis {
        let vulkan_headers_path = vulkan_headers_path.as_ref();
        Analysis {
            vk: Library::new(vulkan_headers_path.join("registry/vk.xml")),
            video: Library::new(vulkan_headers_path.join("registry/video.xml")),
        }
    }

    pub fn dump_as_pseudo_rust(&self) {
        for fp in &self.vk._xml.funcpointers {
            eprintln!(
                "type {} = {};",
                fp.c_decl.name,
                fp.c_decl.ty.to_pseudo_rust()
            );
        }
        for st in &self.vk._xml.structs {
            eprintln!("struct {} {{", st.name);
            for m in &st.members {
                if m.len.len() > 1 {}
                let len = if !m.altlen.is_empty() {
                    &m.altlen
                } else {
                    &m.len
                };
                eprint!("    {}", m.c_decl.to_pseudo_rust_with_external_lengths(len));
                if let Some(val) = &m.values {
                    eprint!(" = {val}");
                }
                eprintln!(",");
            }
            eprintln!("}}");
        }
        for cmd in &self.vk._xml.commands {
            eprintln!("unsafe extern fn {}(", cmd.name);
            for p in &cmd.params {
                let len = if !p.altlen.is_empty() {
                    &p.altlen
                } else {
                    &p.len
                };
                eprint!("    {}", p.c_decl.to_pseudo_rust_with_external_lengths(len));
                eprintln!(",");
            }
            eprint!(")");
            if let Some(ret_ty) = &cmd.return_type {
                eprint!(" -> {}", ret_ty.to_pseudo_rust());
            }
            eprintln!(";");
        }
    }
}

#[derive(Debug)]
pub struct Library {
    _xml: xml::Registry,
}

impl Library {
    fn new(xml_path: impl AsRef<Path>) -> Library {
        let xml = error_span!("xml", path = %xml_path.as_ref().display()).in_scope(|| {
            // We leak the input string here for convenience, to avoid explicit lifetimes.
            let xml_input = Box::leak(fs::read_to_string(xml_path).unwrap().into_boxed_str());
            debug!("parsing xml");
            xml::Registry::parse(xml_input, "vulkan")
        });

        Library { _xml: xml }
    }
}
