use crate::{
    decl::{ArrayLen, CPrimaryType, Decl, Mutability, Ty},
    name::{ConstantName, TypeName},
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Ident;

pub trait NameTranslate {
    fn variable_to_rust(&self, raw: &'static str) -> Ident;

    fn type_to_rust(&self, name: TypeName) -> TokenStream;

    fn constant_to_rust(&self, name: ConstantName) -> TokenStream;

    fn base_type_to_rust(&self, base_ty: CPrimaryType) -> TokenStream {
        match base_ty {
            CPrimaryType::Void => quote! { core::ffi::c_void },
            CPrimaryType::Char => quote! { core::ffi::c_char },
            CPrimaryType::Int => quote! { core::ffi::c_int },
            CPrimaryType::Float => quote! { core::ffi::c_float },
            CPrimaryType::Double => quote! { core::ffi::c_double },
            CPrimaryType::Int8 => quote! { i8 },
            CPrimaryType::UInt8 => quote! { u8 },
            CPrimaryType::Int16 => quote! { i16 },
            CPrimaryType::UInt16 => quote! { u16 },
            CPrimaryType::Int32 => quote! { i32 },
            CPrimaryType::UInt32 => quote! { u32 },
            CPrimaryType::Int64 => quote! { i64 },
            CPrimaryType::UInt64 => quote! { u64 },
            CPrimaryType::Size => quote! { usize },
        }
    }

    fn ext_type_to_rust(&self, raw: &'static str) -> TokenStream;
}

impl Decl {
    /// Gives you this declaration in the form of `#name: #ty`.
    pub fn to_rust(&self, name_translate: &impl NameTranslate) -> TokenStream {
        let name = name_translate.variable_to_rust(self.name);
        let ty = self.ty.to_rust(name_translate);
        quote! { #name: #ty }
    }
}

impl Ty {
    pub fn to_rust(&self, name_translate: &impl NameTranslate) -> TokenStream {
        match self {
            Ty::Spec(name) => name_translate.type_to_rust(*name),
            Ty::CPrimary(base_ty) => name_translate.base_type_to_rust(*base_ty),
            Ty::External(external) => name_translate.ext_type_to_rust(external),
            Ty::Ptr(Ty::Func { ret_ty, params }, _mutability) => {
                let ret = ret_ty.map(|ty| {
                    let ty = ty.to_rust(name_translate);
                    quote! { -> #ty }
                });

                let params = params.iter().map(|decl| decl.to_rust(name_translate));

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
            Ty::Array(ty, array_len) => {
                let ty = ty.to_rust(name_translate);
                let array_len = match array_len {
                    ArrayLen::Constant(constant) => name_translate.constant_to_rust(*constant),
                    ArrayLen::Literal(value) => {
                        let literal = Literal::u128_unsuffixed(*value);
                        quote! { #literal }
                    }
                };

                quote! { [#ty; #array_len] }
            }
        }
    }
}
