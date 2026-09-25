mod item;
pub mod loader;
mod output;

use crate::output::CodeMap;
use analysis::{
    Analysis, AnalysisResult,
    item::RequiredBy,
    name::{
        CMacroName, CommandName, ConstantName, EnumeratorName, FuncPointerName, TypeName,
        VariableName,
    },
    rust::{Lifetime, RustTokens},
};
use heck::{ToShoutySnekCase, ToSnekCase};
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use regex::Regex;
use std::{borrow::Cow, fmt::Display, io, ops::Deref, path::Path, sync::LazyLock};
use syn::Ident;
use tracing::debug;

pub fn generate(analysis: &Analysis, output_path: impl AsRef<Path>) -> io::Result<()> {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    let ctx = Context(analysis.result());
    item::generate_code(&ctx, &mut codemap);
    loader::generate_code(&ctx, &mut codemap);

    codemap.write(&ctx, output_path)
}

fn generate_target_doc_comment(target: &str, description: impl Display) -> String {
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

fn expand_doc_comment(doc: &str) -> Cow<'_, str> {
    static DOC_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<<([\w-]+)>>").unwrap());
    DOC_LINK.replace_all(
        doc,
        // There is no trivial docs.vulkan.org link, but the only existing <#devsandqueues-lost-device>
        // seems to require a link to https://docs.vulkan.org/spec/latest/chapters/devsandqueues.html#devsandqueues-lost-device.
        "<https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html#${1}>",
    )
}

/// Tries to prepend an underscore in case the name is not a valid identifier
fn escape_ident(name: &str) -> Ident {
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

impl Context<'_> {
    fn variable_token_from_original(&self, original: &str) -> Ident {
        crate::escape_ident(&original.to_snek_case())
    }

    fn type_required_by(&self, name: TypeName) -> RequiredBy {
        let type_item = &self.items.types[&name];
        type_item.required_by(self.items)
    }

    fn constant_token_from_prefix_stripped(&self, prefix_stripped: &str) -> Ident {
        if let Some(without_extension) = prefix_stripped.strip_suffix("EXTENSION_NAME") {
            syn::parse_str(&format!("{without_extension}NAME")).unwrap()
        } else {
            syn::parse_str(prefix_stripped).unwrap()
        }
    }

    fn constant_token(&self, name: ConstantName) -> Ident {
        let constant_item = &self.items.constants[&name];
        self.constant_token_from_prefix_stripped(
            name.prefix_stripped(constant_item.required_by.library),
        )
    }
}

impl RustTokens for Context<'_> {
    fn variable_token(&self, name: VariableName) -> Ident {
        self.variable_token_from_original(name.original())
    }

    fn type_tokens(&self, name: TypeName, qualified: bool, lifetime: &Lifetime) -> TokenStream {
        let required_by = self.type_required_by(name);
        let ident: Ident = syn::parse_str(name.prefix_stripped(required_by.library)).unwrap();
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
        let ident = self.constant_token(name);
        let path = qualified.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn enumerator_tokens(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        qualified: bool,
    ) -> TokenStream {
        let prefix = if type_name == TypeName::VK_RESULT {
            String::from("VK_")
        } else {
            let mut prefix = type_name
                .tag_trimmed()
                .replace("FlagBits", "")
                .TO_SHOUTY_SNEK_CASE();

            // add _ before trailing number
            if prefix.ends_with(|c: char| c.is_ascii_digit()) {
                prefix.insert(prefix.len() - 1, '_');
            }

            prefix + "_"
        };

        let prefix_stripped = name.original().strip_prefix(&prefix).unwrap();
        let stripped_name = prefix_stripped.replace("_BIT", "");

        let ident = crate::escape_ident(&stripped_name.TO_SHOUTY_SNEK_CASE());
        let path = qualified.then(|| {
            let bits_name = self.type_tokens(type_name, true, &Lifetime::placeholder());
            quote! { #bits_name:: }
        });

        quote! { #path #ident }
    }

    fn cmacro_tokens(&self, name: CMacroName, qualified: bool) -> TokenStream {
        let ident: Ident = if self.items.cmacros[&name].has_args() {
            syn::parse_str(name.prefix_stripped()).unwrap()
        } else {
            syn::parse_str(&name.prefix_stripped().to_ascii_lowercase()).unwrap()
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
