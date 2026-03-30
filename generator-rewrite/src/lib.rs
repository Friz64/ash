mod item;
pub mod loader;
mod output;

use crate::output::CodeMap;
use analysis::{
    Analysis,
    item::Items,
    name::{
        CMacroName, CommandName, ConstantName, EnumeratorName, FuncPointerName, TypeName,
        VariableName,
    },
    to_rust::RustTranslator,
};
use heck::{ToShoutySnekCase, ToSnekCase};
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use std::{fmt::Display, io, path::Path};
use syn::Ident;
use tracing::debug;

pub fn generate(analysis: &Analysis, output_path: impl AsRef<Path>) -> io::Result<()> {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    let ctx = Context {
        items: analysis.items(),
    };

    item::generate_code(&ctx, &mut codemap);
    loader::generate_code(&ctx, &mut codemap);

    codemap.write(output_path)
}

pub(crate) fn refpage_doc(target: &str, description: impl Display) -> String {
    let valid = matches!(
        target.get(0..2).map(|ab| ab.eq_ignore_ascii_case("vk")),
        Some(true)
    );

    let refpage = if valid {
        format!(
            "[Vulkan Manual Page]\
            (https://docs.vulkan.org/refpages/latest/refpages/source/{target}.html)"
        )
    } else {
        "<s>Vulkan Manual Page</s>".into()
    };

    format!("{} · {}", refpage, description)
}

/// Tries to prepend an underscore in case the name is not a valid identifier
pub(crate) fn escape_ident(name: &str) -> Ident {
    syn::parse_str(name).unwrap_or_else(|_| format_ident!("_{name}"))
}

#[derive(Debug)]
pub struct Context<'a> {
    items: &'a Items,
}

impl<'a> RustTranslator for Context<'a> {
    fn var_name_to_rust(&self, name: VariableName) -> Ident {
        crate::escape_ident(&name.original().to_snek_case())
    }

    fn type_to_rust(&self, name: TypeName, with_path: bool) -> TokenStream {
        let required_by = self.items.types[&name].required_by(self.items);
        let ident: Ident = syn::parse_str(name.prefix_trimmed(required_by.library)).unwrap();
        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn func_pointer_to_rust(&self, name: FuncPointerName, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.original()).unwrap();
        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn command_to_rust(&self, name: CommandName, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(&format!("PFN_{}", name.original())).unwrap();
        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn constant_to_rust(&self, name: ConstantName, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn enumerator_to_rust(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        with_path: bool,
    ) -> TokenStream {
        let ident = crate::escape_ident(&name.stripped(type_name).TO_SHOUTY_SNEK_CASE());
        let path = with_path.then(|| {
            let bits_name = self.type_to_rust(type_name, true);
            quote! { #bits_name:: }
        });

        quote! { #path #ident }
    }

    fn cmacro_to_rust(&self, name: CMacroName, with_path: bool) -> TokenStream {
        let ident: Ident = if self.items.cmacros[&name].has_args() {
            syn::parse_str(name.prefix_trimmed()).unwrap()
        } else {
            syn::parse_str(&name.prefix_trimmed().to_ascii_lowercase()).unwrap()
        };

        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn platform_type_to_rust(&self, raw: &'static str, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(raw).unwrap();
        let path = with_path.then(|| quote! { crate::platform_types:: });
        quote! { #path #ident }
    }
}
