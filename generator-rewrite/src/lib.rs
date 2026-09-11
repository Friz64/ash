mod item;
pub mod loader;
mod output;

use crate::output::CodeMap;
use analysis::{
    Analysis, AnalysisResult,
    lifetime::Lifetime,
    name::{
        CMacroName, CommandName, ConstantName, EnumeratorName, FuncPointerName, TypeName,
        VariableName,
    },
    rust::RustTokens,
};
use heck::{ToShoutySnekCase, ToSnekCase};
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use std::{fmt::Display, io, ops::Deref, path::Path};
use syn::Ident;
use tracing::debug;

pub fn generate(analysis: &Analysis, output_path: impl AsRef<Path>) -> io::Result<()> {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    let ctx = Context(analysis.result());
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
pub struct Context<'a>(&'a AnalysisResult);

impl<'a> Deref for Context<'a> {
    type Target = AnalysisResult;

    fn deref(&self) -> &'a Self::Target {
        self.0
    }
}

impl<'a> RustTokens for Context<'a> {
    fn var_name_token(&self, name: VariableName) -> Ident {
        crate::escape_ident(&name.original().to_snek_case())
    }

    fn type_tokens(&self, name: TypeName, qualified: bool, lifetime: &Lifetime) -> TokenStream {
        let type_item = &self.items.types[&name];
        let required_by = type_item.required_by(&self.items);
        let ident: Ident = syn::parse_str(name.prefix_trimmed(required_by.library)).unwrap();
        let path = qualified.then(|| quote! { crate::vk:: });
        let lifetime = self.type_has_lifetime(name).then(|| quote! { <#lifetime> });
        quote! { #path #ident #lifetime }
    }

    fn func_pointer_tokens(&self, name: FuncPointerName, qualified: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.original()).unwrap();
        let path = qualified.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn command_tokens(&self, name: CommandName, qualified: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(&format!("PFN_{}", name.original())).unwrap();
        let path = qualified.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn constant_tokens(&self, name: ConstantName, qualified: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        let path = qualified.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn enumerator_tokens(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        qualified: bool,
    ) -> TokenStream {
        let ident = crate::escape_ident(&name.stripped(type_name).TO_SHOUTY_SNEK_CASE());
        let path = qualified.then(|| {
            let bits_name = self.type_tokens(type_name, true, &Lifetime::placeholder());
            quote! { #bits_name:: }
        });

        quote! { #path #ident }
    }

    fn cmacro_tokens(&self, name: CMacroName, qualified: bool) -> TokenStream {
        let ident: Ident = if self.items.cmacros[&name].has_args() {
            syn::parse_str(name.prefix_trimmed()).unwrap()
        } else {
            syn::parse_str(&name.prefix_trimmed().to_ascii_lowercase()).unwrap()
        };

        let path = qualified.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn platform_type_tokens(&self, raw: &str, qualified: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(raw).unwrap();
        let path = qualified.then(|| quote! { crate::platform_types:: });
        quote! { #path #ident }
    }
}
