use crate::output::CodeMap;
use analysis::{
    item::{Items, TypeItem},
    name::{CMacroName, ConstantName, EnumeratorName, FuncPointerName, TypeName, VariableName},
    to_rust::RustTranslator,
};
use heck::{ToShoutySnekCase, ToSnekCase};
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
            TypeItem::BitMaskBits { .. } => CodeMap::default(), // covered by `TypeItem::BitMask``
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
pub struct Context<'a> {
    items: &'a Items,
}

impl<'a> RustTranslator for Context<'a> {
    fn var_name_to_rust(&self, name: VariableName) -> Ident {
        crate::escape_ident(&name.original().to_snek_case())
    }

    fn type_to_rust(&self, name: TypeName, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        let path = with_path.then(|| quote! { crate::vk:: });
        quote! { #path #ident }
    }

    fn func_pointer_to_rust(&self, name: FuncPointerName, with_path: bool) -> TokenStream {
        let ident: Ident = syn::parse_str(name.original()).unwrap();
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

pub fn generate_code(items: &Items, codemap: &mut CodeMap) {
    debug!("generating structures code");
    let ctx = Context { items };
    codemap.extend_from_items(&ctx, items.types.values());
    codemap.extend_from_items(&ctx, items.func_pointers.values());
    codemap.extend_from_items(&ctx, items.constants.values());
    codemap.extend_from_items(&ctx, items.cmacros.values());
}
