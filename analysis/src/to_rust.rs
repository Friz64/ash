use crate::{
    decl::{CBaseTy, Decl, Mutability, Ty},
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
            CBaseTy::Char => quote! { core::ffi::c_char },
            CBaseTy::Int => quote! { core::ffi::c_int },
            CBaseTy::Float => quote! { core::ffi::c_float },
            CBaseTy::Double => quote! { core::ffi::c_double },
            CBaseTy::Int8 => quote! { i8 },
            CBaseTy::UInt8 => quote! { u8 },
            CBaseTy::Int16 => quote! { i16 },
            CBaseTy::UInt16 => quote! { u16 },
            CBaseTy::Int32 => quote! { i32 },
            CBaseTy::UInt32 => quote! { u32 },
            CBaseTy::Int64 => quote! { i64 },
            CBaseTy::UInt64 => quote! { u64 },
            CBaseTy::Size => quote! { usize },
        }
    }

    fn ext_type_to_rust(&self, raw: &'static str) -> TokenStream;
}

impl Decl {
    pub fn to_rust_name_colon_type(&self, name_translate: &impl NameTranslate) -> TokenStream {
        let ident = name_translate.variable_to_rust(self.name);
        let ty = self.ty.to_rust(name_translate);
        quote! { #ident: #ty }
    }
}

impl Ty {
    pub fn to_rust(&self, name_translate: &impl NameTranslate) -> TokenStream {
        match self {
            Ty::Spec(name) => name_translate.spec_type_to_rust(*name),
            Ty::CBase(base_ty) => name_translate.base_type_to_rust(*base_ty),
            Ty::External(external) => name_translate.ext_type_to_rust(external),
            Ty::Ptr(Ty::Func { ret_ty, params }, _mutability) => {
                let ret = ret_ty.map(|ty| {
                    let ty = ty.to_rust(name_translate);
                    quote! { -> #ty }
                });

                let params = params
                    .iter()
                    .map(|decl| decl.to_rust_name_colon_type(name_translate));

                quote! { unsafe extern "system" fn( #( #params ),* ) #ret }
            }
            Ty::Func { .. } => panic!("A pointed-to function cannot exist without a pointer"),
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
        }
    }
}
