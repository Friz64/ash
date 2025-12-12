use std::ops::Deref;

use crate::{item, output::CodeMap, util};
use analysis::{
    decl::{BaseTy, Mutability, Ty},
    item::{ItemInfo, Items},
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use tracing::debug;

mod structure;

pub trait Code {
    fn code(&self) -> CodeMap;
}

impl CodeMap {
    pub fn extend_from_items<'a, C: Code + 'a>(
        &mut self,
        item_iter: impl IntoIterator<Item = &'a C>,
    ) {
        for item in item_iter {
            self.extend(item.code());
        }
    }
}

fn ty_to_rust(ty: &Ty) -> TokenStream {
    match ty {
        Ty::Item(item) => {
            let ident: Ident = syn::parse_str(item.name()).unwrap();
            quote! { crate::vk::#ident }
        }
        Ty::Base(base_ty) => match base_ty {
            BaseTy::Void => quote! { core::ffi::c_void },
        },
        Ty::External(external) => quote! { crate::External<{ #external; 0 }> },
        Ty::Ptr(ty, mutability) => {
            let mutability = match mutability {
                Mutability::Not => quote! { const },
                Mutability::Mut => quote! { mut },
            };

            let ty = ty_to_rust(ty);
            quote! { * #mutability #ty }
        }
        Ty::Ref(ty, mutability) => {
            let mutability = match mutability {
                Mutability::Not => quote! {},
                Mutability::Mut => quote! { mut },
            };

            let ty = ty_to_rust(ty);
            quote! { & #mutability #ty }
        }
        Ty::Array(ty, _array_len) => {
            let ty = ty_to_rust(ty);
            quote! { [#ty; 1337] }
        }
        Ty::Func { ret_ty, params } => {
            let ret = ret_ty.map(|ty| {
                let ty = ty_to_rust(ty);
                quote! { -> #ty }
            });

            let params = params.iter().map(|decl| {
                let name = util::to_snake_case_escape_ident(decl.name);
                let ty = item::ty_to_rust(&decl.ty);
                quote! { #name: #ty, }
            });

            quote! { unsafe extern "system" fn( #( #params )* ) #ret }
        }
    }
}

pub fn build_items_codemap(items: &Items) -> CodeMap {
    debug!("building codemap");
    let mut codemap = CodeMap::default();

    debug!("generating structures code");
    codemap.extend_from_items(items.structures.iter().map(Deref::deref));

    codemap
}
