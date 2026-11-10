mod vfs;

use crate::{Context, output::vfs::VirtualRustFs};
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
    cell::{Ref, RefCell, RefMut},
    hash::Hash,
    io, iter,
    path::{Path, PathBuf},
};
use syn::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobExport {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutputLocation {
    pub library: LibraryName,
    pub require: RequireLocation,
    pub glob_export: GlobExport,
}

impl OutputLocation {
    pub fn primary_of(required_by: RequiredBy, glob_export: GlobExport) -> OutputLocation {
        OutputLocation {
            library: required_by.library,
            require: required_by.primary_location(),
            glob_export,
        }
    }

    pub fn all_of(
        required_by: RequiredBy,
        glob_export: GlobExport,
    ) -> impl Iterator<Item = OutputLocation> {
        (required_by.locations)
            .into_iter()
            .map(move |require| OutputLocation {
                library: required_by.library,
                require,
                glob_export,
            })
    }

    pub fn provided_by_doc_comment(&self) -> String {
        let friendly_name = match self.require {
            RequireLocation::Core { major, minor } => format!("Vulkan {major}.{minor}"),
            RequireLocation::Extension { name } => name.original().into(),
        };

        let components = PathComponent::list(self.library, self.require)
            .into_iter()
            .map(|component| component.module_name.to_string())
            .join("::");

        format!("Provided by [{friendly_name}](crate::{components})")
    }
}

struct PathComponent {
    module_name: Ident,
    doc_comment: String,
}

impl PathComponent {
    fn list(library: LibraryName, require_location: RequireLocation) -> Vec<PathComponent> {
        let original_name = require_location.original_name();
        match require_location {
            RequireLocation::Core { major, minor } => vec![PathComponent {
                module_name: format_ident!("vk{major}_{minor}"),
                doc_comment: crate::target_manual_page_doc_comment(
                    &original_name,
                    format!("Vulkan version {major}.{minor}"),
                ),
            }],
            RequireLocation::Extension { name } => match library {
                LibraryName::Vk => {
                    let (ext_tag, ext_name) = name.tag_and_name(library);
                    vec![
                        PathComponent {
                            module_name: format_ident!("{}", ext_tag.to_ascii_lowercase()),
                            doc_comment: format!("Extensions tagged {ext_tag}"),
                        },
                        PathComponent {
                            module_name: crate::escape_ident(&ext_name.to_snek_case()),
                            doc_comment: crate::target_manual_page_doc_comment(
                                &original_name,
                                format!("Extension `{}`", original_name),
                            ),
                        },
                    ]
                }
                LibraryName::Video => {
                    let prefix_stripped = name.prefix_stripped(library);
                    vec![
                        PathComponent {
                            module_name: format_ident!("video"),
                            doc_comment: "Vulkan Video".into(),
                        },
                        PathComponent {
                            module_name: format_ident!("{prefix_stripped}"),
                            doc_comment: format!("Items provided by `{}`", original_name),
                        },
                    ]
                }
            },
        }
    }
}

struct ReexportAs {
    original: Ident,
    alias: Ident,
}

#[derive(Default)]
struct Code {
    tokens: TokenStream,
    reexports: Vec<ReexportAs>,
}

#[derive(Default)]
pub struct CodeMap(IndexMap<OutputLocation, Code>);

impl CodeMap {
    pub fn octx<'c, 'm>(
        &'m mut self,
        output: OutputLocation,
        ctx: &'c Context<'c>,
    ) -> OutputContext<'c, 'm> {
        OutputContext(RefCell::new(OutputContextInner {
            ctx,
            codemap: self,
            output,
        }))
    }

    pub fn extend_mapped<'c, 'm, F>(&'m mut self, other: Self, ctx: &'c Context, mut map_fn: F)
    where
        F: for<'f> FnMut(OutputContext<'c, 'f>, TokenStream) -> TokenStream,
    {
        for (output, code) in other.0 {
            let octx = self.octx(output, ctx);
            let tokens = map_fn(octx, code.tokens);

            let entry = self.0.entry(output).or_default();
            entry.tokens.extend(tokens);
        }
    }
}

struct OutputContextInner<'c, 'm> {
    ctx: &'c Context<'c>,
    codemap: &'m mut CodeMap,
    output: OutputLocation,
}

pub struct OutputContext<'c, 'm>(RefCell<OutputContextInner<'c, 'm>>);

impl<'c, 'm> OutputContext<'c, 'm> {
    pub fn ctx(&self) -> Ref<'_, Context<'c>> {
        Ref::map(self.0.borrow(), |inner| inner.ctx)
    }

    pub fn output(&self) -> RefMut<'_, OutputLocation> {
        RefMut::map(self.0.borrow_mut(), |inner| &mut inner.output)
    }

    pub fn emit_tokens(&self, tokens: TokenStream) {
        let mut inner = self.0.borrow_mut();
        let output = inner.output;
        let entry = inner.codemap.0.entry(output).or_default();
        entry.tokens.extend(tokens);
    }

    pub fn emit_reexport(&self, original: Ident, alias: Ident) {
        let mut inner = self.0.borrow_mut();
        std::assert_matches!(inner.output.glob_export, GlobExport::No);
        let output = inner.output;
        let entry = inner.codemap.0.entry(output).or_default();
        entry.reexports.push(ReexportAs { original, alias });
    }
}

impl CodeMap {
    pub fn write(self, ctx: &Context, output_path: impl AsRef<Path>) -> io::Result<()> {
        let mut vfs = VirtualRustFs::default();
        vfs.write(
            "mod.rs",
            quote! {
                /// A re-export of all items
                pub mod vk;
                mod prelude {
                    pub use crate::{vk, Extends, Handle, TaggedStructure, VkResult};
                    pub use core::ffi::{self, CStr};
                    pub use core::fmt;
                    pub use core::marker::PhantomData;
                    pub use core::mem::{transmute, zeroed, MaybeUninit};
                    pub use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};
                    pub use core::ptr;
                    #[cfg(feature = "debug")]
                    pub use core::fmt as fmtd;
                    pub use alloc::vec::Vec;
                }
            },
        );
        vfs.write(
            "vk.rs",
            quote! {
                pub use crate::platform_types::*;
                pub use crate::Handle;
                pub use crate::TaggedStructure;
                pub use crate::Extends;
            },
        );

        struct ModFile {
            doc_comment: Option<String>,
            child_modules: IndexMap<Ident, RequireLocation>,
        }

        struct SourceFile {
            library: LibraryName,
            require_location: RequireLocation,
            doc_comment: String,
            /// e.g. VK_EXT_FOO for easy lookup
            doc_alias: String,
            /// e.g. `[NAME as FOO_NAME, SPEC_VERSION as FOO_SPEC_VERSION]`
            reexport_items: Vec<TokenStream>,
            /// items that will be glob-reexported to the vk module
            reexport_content: TokenStream,
            /// other items
            content: TokenStream,
        }

        let mut mod_files: IndexMap<PathBuf, ModFile> = Default::default();
        let mut source_files: IndexMap<PathBuf, SourceFile> = Default::default();
        for (output, code) in self.0 {
            let components = PathComponent::list(output.library, output.require);

            let doc = &components.last().unwrap().doc_comment;
            let mut path = PathBuf::from_iter(
                (components.iter())
                    .map(|component| PathBuf::from(component.module_name.to_string())),
            );
            path.add_extension("rs");

            let source_file = source_files.entry(path).or_insert_with(|| SourceFile {
                library: output.library,
                require_location: output.require,
                doc_comment: doc.into(),
                doc_alias: output.require.original_name(),
                reexport_items: Vec::new(),
                reexport_content: TokenStream::new(),
                content: TokenStream::new(),
            });

            let oooooh_im_preluuuding = quote! { use crate::generated::prelude::*; };
            if output.glob_export == GlobExport::Yes {
                source_file.reexport_content.extend(oooooh_im_preluuuding);
                source_file.reexport_content.extend(code.tokens);
            } else {
                source_file.content.extend(oooooh_im_preluuuding);
                source_file.content.extend(code.tokens);
            };

            source_file.reexport_items.extend(
                code.reexports
                    .iter()
                    .map(|ReexportAs { original, alias }| quote! { #original as #alias }),
            );

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
                    child_modules: IndexMap::new(),
                });

                (mod_file.child_modules).insert_sorted_by_key(
                    component.module_name.clone(),
                    output.require,
                    |_k, v| *v,
                );
            }
        }

        for (source_path, source_file) in source_files {
            let SourceFile {
                library,
                require_location,
                doc_comment,
                doc_alias,
                mut reexport_items,
                mut reexport_content,
                content,
            } = source_file;

            let is_provisional = require_location.is_provisional(ctx);
            let provisional_mod_guard =
                is_provisional.then_some(quote! { #![cfg(feature = "provisional")] });
            let provisional_guard =
                is_provisional.then_some(quote! { #[cfg(feature = "provisional")] });

            if !reexport_content.is_empty() {
                reexport_content = quote! {
                    pub(crate) mod items { #reexport_content }
                    pub use items::*;
                };

                reexport_items.push(quote! { items::* });
            }

            let components = PathComponent::list(library, require_location);
            let component_idents: Vec<_> = components
                .iter()
                .map(|component| &component.module_name)
                .collect();

            let vk_content = reexport_items.into_iter().map(|reexport_item| {
                // this is quite difficult to parse by eye:
                // #component_idents is using repetition syntax #()*
                quote! {
                    #provisional_guard
                    pub use super:: #(#component_idents)::* ::#reexport_item;
                }
            });

            vfs.write("vk.rs", quote! { #(#vk_content)* });

            vfs.write(
                source_path,
                quote! {
                    #provisional_mod_guard
                    #![doc = #doc_comment]
                    #![doc(alias = #doc_alias)]
                    #content
                    #reexport_content
                },
            );
        }

        for (mod_path, mod_file) in mod_files {
            let doc = mod_file.doc_comment.map(|doc| quote! { #![doc = #doc] });
            let mod_child_idents = mod_file.child_modules.keys();
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
