use crate::output::CodeMap;
use analysis::{
    item::{Items, TypeItem},
    name::TypeName,
    to_rust::NameTranslate,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use tracing::debug;

mod basetype;
mod enumeration;
mod handle;
mod structure;

pub trait Code {
    fn code(&self, ctx: &Context) -> CodeMap;
}

impl Code for TypeItem {
    fn code(&self, ctx: &Context) -> CodeMap {
        match self {
            TypeItem::Structure(structure) => structure.code(ctx),
            TypeItem::Enumeration(enumeration) => enumeration.code(ctx),
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

pub struct Context {}

impl NameTranslate for Context {
    fn variable_to_rust(&self, raw: &'static str) -> Ident {
        crate::to_snake_case_escape_ident(raw)
    }

    fn spec_type_to_rust(&self, name: TypeName) -> TokenStream {
        let ident: Ident = syn::parse_str(name.prefix_trimmed()).unwrap();
        quote! { crate::vk::#ident }
    }

    fn ext_type_to_rust(&self, raw: &'static str) -> TokenStream {
        quote! { crate::External<{ #raw; 0 }> }
    }
}

pub fn build_items_codemap(items: &Items) -> CodeMap {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    debug!("generating structures code");
    let ctx = Context {};
    codemap.extend_from_items(&ctx, items.types.values());

    codemap
}
