use crate::{
    decl::{ArrayLen, CPrimaryType, Decl, Mutability, Ty},
    item::constant,
    name::{ConstantName, FuncPointerName, TypeName},
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Ident;

pub trait NameTranslate {
    fn variable_to_rust(&self, raw: &'static str) -> Ident;

    fn type_to_rust(&self, name: TypeName) -> TokenStream;

    fn func_pointer_to_rust(&self, name: FuncPointerName) -> TokenStream;

    fn constant_to_rust(&self, name: ConstantName) -> TokenStream;

    fn primary_type_to_rust(&self, primary_ty: CPrimaryType) -> TokenStream {
        match primary_ty {
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
            Ty::SpecType(name) => name_translate.type_to_rust(*name),
            Ty::SpecFuncPointer(name) => name_translate.func_pointer_to_rust(*name),
            Ty::CPrimary(base_ty) => name_translate.primary_type_to_rust(*base_ty),
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
            Ty::Array(ty, array_len) => {
                let ty = ty.to_rust(name_translate);
                let array_len = match array_len {
                    ArrayLen::Constant(constant) => name_translate.constant_to_rust(*constant),
                    ArrayLen::Literal(value) => {
                        let literal = Literal::u128_unsuffixed(*value);
                        quote! { #literal }
                    }
                };

                quote! { [#ty; #array_len as _] }
            }
        }
    }
}

impl constant::Expression {
    pub fn to_rust(&self, _name_translate: &impl NameTranslate) -> TokenStream {
        let mut rust_expr = String::default();

        let mut s = self.0;
        assert!(s.is_ascii());
        while let Some(c) = s.chars().next() {
            let is_value = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
            let rust_token = if is_value(c) {
                let len = s.chars().take_while(|&c| is_value(c)).count();
                let (token, rest) = s.split_at(len);
                s = rest;
                if c.is_ascii_digit() {
                    // `token` is  a literal integer
                    let mut horrible_code_pls_fix = token.replace("ULL", "u64").replace("U", "u32");

                    if horrible_code_pls_fix.contains('.') {
                        horrible_code_pls_fix = horrible_code_pls_fix
                            .replace("f", "f32")
                            .replace("F", "f32");
                    }
                    horrible_code_pls_fix
                } else {
                    // `token` is a macro invocation
                    String::from("69420") // TODO
                }
            } else if c.is_ascii_punctuation() {
                s = &s[1..];
                // `c` is punctuation
                if c == '~' {
                    String::from('!')
                } else {
                    c.to_string()
                }
            } else if c.is_ascii_whitespace() {
                s = s.trim_start();
                continue;
            } else {
                panic!("unsupported token: {c:?}");
            };

            rust_expr += &(rust_token + " ");
        }

        let tokens = syn::parse_str(&rust_expr).unwrap();
        syn::parse2::<syn::File>(quote! { const WAFF: usize = #tokens; }).unwrap();
        tokens
    }
}
