use crate::{
    decl::{CBaseTy, Mutability, Ty},
    name::TypeName,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub trait NameTranslate {
    fn variable_to_rust(&self, raw: &'static str) -> Ident;

    fn spec_type_to_rust(&self, name: TypeName) -> TokenStream;

    fn base_type_to_rust(&self, base_ty: CBaseTy) -> TokenStream {
        match base_ty {
            CBaseTy::Void => quote! { core::ffi::c_void },
        }
    }

    fn ext_type_to_rust(&self, raw: &'static str) -> TokenStream;
}

impl Ty {
    pub fn to_rust(&self, name_translate: &impl NameTranslate) -> TokenStream {
        match self {
            Ty::Spec(name) => name_translate.spec_type_to_rust(*name),
            Ty::CBase(base_ty) => name_translate.base_type_to_rust(*base_ty),
            Ty::External(external) => name_translate.ext_type_to_rust(external),
            Ty::Ptr(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! { const },
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(name_translate);
                quote! { * #mutability #ty }
            }
            Ty::Ref(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(name_translate);
                quote! { & #mutability #ty }
            }
            Ty::Array(ty, _array_len) => {
                let ty = ty.to_rust(name_translate);
                quote! { [#ty; 1337] }
            }
            Ty::Func { ret_ty, params } => {
                let ret = ret_ty.map(|ty| {
                    let ty = ty.to_rust(name_translate);
                    quote! { -> #ty }
                });

                let params = params.iter().map(|decl| {
                    let name = name_translate.variable_to_rust(decl.name);
                    let ty = decl.ty.to_rust(name_translate);
                    quote! { #name: #ty }
                });

                quote! { unsafe extern "system" fn( #( #params ),* ) #ret }
            }
        }
    }
}
