mod vfs;

use crate::output::vfs::VirtualRustFs;
use analysis::{
    LibraryName,
    item::{RequireLocation, RequiredBy},
};
use heck::ToSnekCase;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{
    hash::Hash,
    io, iter,
    path::{Path, PathBuf},
};
use syn::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Destination {
    pub library: LibraryName,
    pub location: RequireLocation,
    pub reexport: bool,
}

impl Destination {
    pub fn primary_location(required_by: RequiredBy) -> Destination {
        Destination {
            library: required_by.library,
            location: required_by.primary_location(),
            reexport: true,
        }
    }

    pub fn all_locations(required_by: RequiredBy) -> impl Iterator<Item = Destination> {
        (required_by.locations)
            .into_iter()
            .map(move |location| Destination {
                library: required_by.library,
                location,
                reexport: true,
            })
    }
}

struct DestinationPathComponent {
    module_name: Ident,
    doc_comment: String,
}

impl Destination {
    fn original_name(&self) -> String {
        match self.location {
            RequireLocation::Core { major, minor } => format!("VK_VERSION_{major}_{minor}"),
            RequireLocation::Extension { name } => name.into(),
        }
    }

    fn path_components(&self) -> Vec<DestinationPathComponent> {
        let original_name = self.original_name();
        match self.location {
            RequireLocation::Core { major, minor } => vec![DestinationPathComponent {
                module_name: format_ident!("vk{major}_{minor}"),
                doc_comment: crate::refpage_doc(
                    &original_name,
                    format!("Vulkan version {major}.{minor}"),
                ),
            }],
            RequireLocation::Extension { name } => match self.library {
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
                            doc_comment: crate::refpage_doc(
                                &original_name,
                                format!("Extension `{name}`"),
                            ),
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

#[derive(Default)]
pub struct CodeMap(IndexMap<Destination, TokenStream>);

impl CodeMap {
    pub fn new(dest: Destination, tokens: TokenStream) -> Self {
        let mut map = IndexMap::with_capacity(1);
        map.insert(dest, tokens);
        CodeMap(map)
    }

    pub fn extend(&mut self, other: Self) {
        for (destination, tokens) in other.0 {
            self.0.entry(destination).or_default().extend(tokens);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Destination, &TokenStream)> {
        self.0.iter()
    }

    pub fn write(&self, output_path: impl AsRef<Path>) -> io::Result<()> {
        let mut vfs = VirtualRustFs::default();
        vfs.write(
            "mod.rs",
            quote! {
                /// A re-export of all items
                pub mod vk;
            },
        );

        struct ModFile {
            doc_comment: Option<String>,
            child_modules: IndexSet<Ident>,
        }

        struct SourceFile {
            destination: Destination,
            doc_comment: String,
            doc_alias: String,
            reexport_content: TokenStream,
            content: TokenStream,
        }

        let mut mod_files: IndexMap<PathBuf, ModFile> = Default::default();
        let mut source_files: IndexMap<PathBuf, SourceFile> = Default::default();
        for (destination, content) in self.iter() {
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

            let source_file = source_files.entry(path).or_insert_with(|| SourceFile {
                destination: *destination,
                doc_comment: doc.into(),
                doc_alias: destination.original_name(),
                reexport_content: TokenStream::new(),
                content: TokenStream::new(),
            });

            if destination.reexport {
                source_file.reexport_content.extend(content.clone());
            } else {
                source_file.content.extend(content.clone());
            }

            // this affects the order impl blocks show up in rustdoc
            let sort_order = match destination.location {
                RequireLocation::Core { .. } => 1,
                RequireLocation::Extension { .. } => 2,
            };

            // collect mod.rs files to be created
            for (i, component) in components.iter().enumerate() {
                let parent_path = &components[0..i];
                let mod_path = PathBuf::from_iter(
                    (parent_path.iter())
                        .map(|component| PathBuf::from(&component.module_name.to_string()))
                        .chain(iter::once(PathBuf::from("mod.rs"))),
                );

                let mod_file = mod_files.entry(mod_path).or_insert_with(|| ModFile {
                    doc_comment: (parent_path.last())
                        .map(|component| component.doc_comment.clone()),
                    child_modules: IndexSet::new(),
                });

                (mod_file.child_modules)
                    .insert_sorted_by_key(component.module_name.clone(), |_| sort_order);
            }
        }

        for (source_path, source_file) in source_files {
            let SourceFile {
                destination,
                doc_comment,
                doc_alias,
                mut reexport_content,
                content,
            } = source_file;

            if !reexport_content.is_empty() {
                let mut module = None;
                if !content.is_empty() {
                    module = Some(quote! { ::reexport });
                    reexport_content = quote! {
                        pub(crate) mod reexport { #reexport_content }
                        pub use reexport::*;
                    };
                }

                let components = destination.path_components();
                let component_idents = components.iter().map(|component| &component.module_name);
                vfs.write(
                    "vk.rs",
                    quote! { pub use super:: #( #component_idents ) :: * #module ::*; },
                );
            }

            vfs.write(
                source_path,
                quote! {
                    #![doc = #doc_comment]
                    #![doc(alias = #doc_alias)]
                    #content
                    #reexport_content
                },
            );
        }

        for (mod_path, mod_file) in mod_files {
            let doc = mod_file.doc_comment.map(|doc| quote! { #![doc = #doc] });
            let mod_child_idents = mod_file.child_modules.iter();
            vfs.write(
                mod_path,
                quote! {
                    #doc
                    #(pub mod #mod_child_idents;)*
                },
            );
        }

        vfs.write(
            "vk.rs",
            quote! {
                pub use crate::Handle;
                pub use crate::TaggedStructure;
                pub use crate::Extends;
            },
        );

        vfs.sync_to(output_path)
    }
}
