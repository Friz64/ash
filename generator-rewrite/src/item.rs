use crate::output::CodeMap;
use analysis::{
    item::{Items, TypeItem},
    to_rust::RustTranslator,
    xml::name::{CMacroName, ConstantName, FuncPointerName, TypeName},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use tracing::debug;

mod alias;
mod basetype;
mod bitmask;
mod cmacro;
mod constant;
mod enumeration;
mod function;
mod handle;
mod structure;

pub trait Code {
    fn code(&self, ctx: &Context) -> CodeMap;
}

impl Code for TypeItem {
    fn code(&self, ctx: &Context) -> CodeMap {
        match self {
            TypeItem::Alias(alias) => alias.code(ctx),
            TypeItem::Struct(structure) => structure.code(ctx),
            TypeItem::Union(union) => union.code(ctx),
            TypeItem::Enum(enumeration) => enumeration.code(ctx),
            TypeItem::BitMask(bitmask) => bitmask.code(ctx),
            TypeItem::BitMaskBits(bitmask_bits) => bitmask_bits.code(ctx),
            TypeItem::BaseType(basetype) => basetype.code(ctx),
            TypeItem::Handle(handle) => handle.code(ctx),
        }
    }
}

impl CodeMap {
    pub fn extend_from_items<'a, C: Code + 'a>(
        &mut self,
        ctx: &Context,
        item_iter: impl IntoIterator<Item = &'a C>,
    ) {
        for item in item_iter {
            self.extend(item.code(ctx));
        }
    }
}

#[derive(Debug)]
pub struct Context {}

impl RustTranslator for Context {
    fn var_name_to_rust(&self, raw: &'static str) -> Ident {
        crate::to_snake_case_escape_ident(raw)
    }

    fn type_to_rust(&self, name: TypeName) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        quote! { crate::vk::#ident }
    }

    fn func_pointer_to_rust(&self, name: FuncPointerName) -> TokenStream {
        let ident: Ident = syn::parse_str(name.original()).unwrap();
        quote! { crate::vk::#ident }
    }

    fn constant_to_rust(&self, name: ConstantName) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        quote! { crate::vk::#ident }
    }

    fn cmacro_to_rust(&self, name: CMacroName, has_args: bool) -> TokenStream {
        let ident: Ident = if has_args {
            syn::parse_str(&name.prefix_trimmed_lowercase()).unwrap()
        } else {
            syn::parse_str(name.prefix_trimmed()).unwrap()
        };

        quote! { crate::vk::#ident }
    }

    fn ext_type_to_rust(&self, raw: &'static str) -> TokenStream {
        let ident: Ident = syn::parse_str(raw).unwrap();
        quote! { crate::platform_types::#ident }
    }
}

pub fn generate_code(items: &Items, codemap: &mut CodeMap) {
    debug!("generating structures code");
    let ctx = Context {};
    codemap.extend_from_items(&ctx, items.types.values());
    codemap.extend_from_items(&ctx, items.func_pointers.values());
    codemap.extend_from_items(&ctx, items.constants.values());
    codemap.extend_from_items(&ctx, items.cmacros.values());
}
