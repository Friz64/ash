mod vfs;

use crate::output::vfs::VirtualRustFs;
use analysis::{
    LibraryName,
    item::{RequireLocation, RequiredBy},
};
use heck::ToSnekCase;
use indexmap::IndexMap;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    io, iter,
    path::{Path, PathBuf},
};
use syn::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Destination {
    Library {
        library: LibraryName,
        location: RequireLocation,
    },
    Loader,
}

impl Destination {
    pub fn library(required_by: RequiredBy) -> Destination {
        Destination::Library {
            library: required_by.library,
            // TODO: figure out secondary locations
            // we need to be generating type aliases or smth like that at those...
            location: required_by.primary_location(),
        }
    }
}

struct DestinationPathComponent {
    module_name: Ident,
    doc_comment: String,
}

impl Destination {
    fn path_components(&self) -> Vec<DestinationPathComponent> {
        match self {
            Destination::Loader => vec![],
            Destination::Library {
                location: RequireLocation::Core { major, minor },
                ..
            } => vec![DestinationPathComponent {
                module_name: format_ident!("vk{major}_{minor}"),
                doc_comment: crate::refpage_doc(
                    &format!("VK_VERSION_{major}_{minor}"),
                    format!("Vulkan version {major}.{minor}"),
                ),
            }],
            Destination::Library {
                location: RequireLocation::Extension { name },
                library,
            } => match library {
                LibraryName::Vk => {
                    let vulkan_ext = name.strip_prefix("VK_").unwrap();
                    let (ext_tag, ext_name) = vulkan_ext.split_once('_').unwrap();
                    vec![
                        DestinationPathComponent {
                            module_name: format_ident!("{}", ext_tag.to_ascii_lowercase()),
                            doc_comment: format!("Extensions tagged {ext_tag}"),
                        },
                        DestinationPathComponent {
                            module_name: crate::escape_ident(&ext_name.to_snek_case()),
                            doc_comment: crate::refpage_doc(name, format!("Extension `{name}`")),
                        },
                    ]
                }
                LibraryName::Video => {
                    let video_ext = name.strip_prefix("vulkan_video_").unwrap();
                    vec![
                        DestinationPathComponent {
                            module_name: format_ident!("video"),
                            doc_comment: "Vulkan Video".into(),
                        },
                        DestinationPathComponent {
                            module_name: format_ident!("{video_ext}"),
                            doc_comment: format!("Items provided by `{}`", name),
                        },
                    ]
                }
            },
        }
    }

    pub fn doc_link(&self) -> String {
        let components = (self.path_components().iter())
            .map(|component| component.module_name.to_string())
            .join("::");
        format!("Provided by [`{components}`](crate::{components})")
    }
}

pub struct CodeMap<K = Destination>(IndexMap<K, TokenStream>);

impl<K> Default for CodeMap<K> {
    fn default() -> Self {
        CodeMap(IndexMap::new())
    }
}

impl<K: Hash + Eq> CodeMap<K> {
    pub fn new(key: K, tokens: TokenStream) -> Self {
        let mut map = IndexMap::with_capacity(1);
        map.insert(key, tokens);
        CodeMap(map)
    }

    pub fn extend(&mut self, other: Self) {
        for (destination, tokens) in other.0 {
            self.0.entry(destination).or_default().extend(tokens);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &TokenStream)> {
        self.0.iter()
    }
}

impl CodeMap<Destination> {
    pub fn write(&self, output_path: impl AsRef<Path>) -> io::Result<()> {
        let mut vfs = VirtualRustFs::default();
        vfs.write(
            "mod.rs",
            quote! {
                /// A re-export of all items
                pub mod vk;
            },
        );

        // foo/bar/mod.rs -> (doc comment, child modules)
        let mut mod_files: HashMap<PathBuf, (Option<String>, HashSet<_>)> = Default::default();
        for (destination, content) in self.iter() {
            // This influences the order in which impl blocks show up in rustdoc
            let sort_pref = match destination {
                Destination::Library { location, .. } => match location {
                    RequireLocation::Core { .. } => 0,
                    RequireLocation::Extension { .. } => 1,
                },
                Destination::Loader => 2,
            };

            let components = destination.path_components();
            if components.is_empty() {
                vfs.write("mod.rs", content.clone());
                continue;
            }

            let doc = &components.last().unwrap().doc_comment;
            let mut path = PathBuf::from_iter(
                (components.iter())
                    .map(|component| PathBuf::from(component.module_name.to_string())),
            );
            path.add_extension("rs");

            vfs.write(
                path,
                quote! {
                    #![doc = #doc]
                    #content
                },
            );

            let component_idents = components.iter().map(|component| &component.module_name);
            vfs.write(
                "vk.rs",
                quote! { pub use super:: #( #component_idents ) :: * ::*; },
            );

            // collect mod.rs files to be created
            for (i, component) in components.iter().enumerate() {
                let parent_path = &components[0..i];
                let mod_path = PathBuf::from_iter(
                    (parent_path.iter())
                        .map(|component| PathBuf::from(&component.module_name.to_string()))
                        .chain(iter::once(PathBuf::from("mod.rs"))),
                );

                let (_, mod_child_idents) = mod_files.entry(mod_path).or_insert_with(|| {
                    let doc = (parent_path.last()).map(|component| component.doc_comment.clone());
                    (doc, HashSet::new())
                });

                mod_child_idents.insert((sort_pref, component.module_name.clone()));
            }
        }

        for (mod_path, (doc, mod_child_idents)) in mod_files {
            let mut mod_child_idents: Vec<(i32, Ident)> = mod_child_idents.into_iter().collect();
            mod_child_idents.sort_unstable();
            let mod_child_idents = mod_child_idents.iter().map(|(_sort_pref, ident)| ident);

            let doc = doc.map(|doc| quote! { #![doc = #doc] });
            vfs.write(
                mod_path,
                quote! {
                    #doc
                    #(pub mod #mod_child_idents;)*
                },
            );
        }

        vfs.sync_to(output_path)
    }
}
