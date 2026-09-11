use crate::{
    decl::{ArrayLen, CPrimaryType, Decl, Mutability, Ty},
    lifetime::Lifetime,
    name::{
        CMacroName, CommandName, ConstantName, EnumeratorName, FuncPointerName, TypeName,
        VariableName,
    },
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::{borrow::Borrow, mem};
use syn::Ident;

#[derive(Debug)]
pub struct RustDecl {
    pub name: VariableName,
    pub ty: RustTy,
}

impl Decl {
    pub fn to_rust(&self) -> RustDecl {
        RustDecl {
            name: self.name,
            ty: self.ty.to_rust(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RustTy {
    Registry(Ty),
    Ref(Box<RustTy>, Mutability),
    Slice(Box<RustTy>, Mutability, Option<ArrayLen>),
    CStr,
}

impl Ty {
    pub fn to_rust(&self) -> RustTy {
        RustTy::Registry(self.clone())
    }
}

pub trait RustTokens {
    fn var_name_token(&self, name: VariableName) -> Ident;

    fn type_tokens(&self, name: TypeName, qualified: bool, lifetime: &Lifetime) -> TokenStream;

    fn func_pointer_tokens(&self, name: FuncPointerName, qualified: bool) -> TokenStream;

    fn command_tokens(&self, name: CommandName, qualified: bool) -> TokenStream;

    fn constant_tokens(&self, name: ConstantName, qualified: bool) -> TokenStream;

    fn enumerator_tokens(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        qualified: bool,
    ) -> TokenStream;

    fn cmacro_tokens(&self, name: CMacroName, qualified: bool) -> TokenStream;

    fn platform_type_tokens(&self, raw: &str, qualified: bool) -> TokenStream;

    fn primary_type_tokens(&self, primary_ty: CPrimaryType) -> TokenStream {
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
}

impl RustDecl {
    /// Gives you this declaration in the form of `#name: #ty`.
    pub fn tokens(&self, translator: &impl RustTokens, lifetime: &Lifetime) -> TokenStream {
        let name = translator.var_name_token(self.name);
        let ty = self.ty.tokens(translator, lifetime);
        quote! { #name: #ty }
    }
}

impl ArrayLen {
    pub fn tokens(&self, translator: &impl RustTokens) -> TokenStream {
        match self {
            ArrayLen::Constant(constant_name) => translator.constant_tokens(*constant_name, true),
            ArrayLen::Literal(value) => {
                let literal = Literal::u128_unsuffixed(*value);
                quote::quote! { #literal }
            }
        }
    }
}

impl RustTy {
    pub fn tokens(&self, translator: &impl RustTokens, lifetime: &Lifetime) -> TokenStream {
        match self {
            RustTy::Registry(ty) => match ty {
                Ty::ApiType(name) => translator.type_tokens(*name, true, lifetime),
                Ty::ApiFuncPointer(name) => translator.func_pointer_tokens(*name, true),
                Ty::CPrimary(base_ty) => translator.primary_type_tokens(*base_ty),
                Ty::Platform(raw) => translator.platform_type_tokens(raw, true),
                Ty::Ptr(ty, mutability) => {
                    let mutability = match mutability {
                        Mutability::Not => quote! { const },
                        Mutability::Mut => quote! { mut },
                    };

                    let ty = ty.to_rust().tokens(translator, lifetime);
                    quote! { * #mutability #ty }
                }
                Ty::Array(ty, array_len) => {
                    let ty = ty.to_rust().tokens(translator, lifetime);
                    let array_len = array_len.tokens(translator);
                    quote! { [#ty; #array_len as _] }
                }
            },
            RustTy::Ref(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.tokens(translator, lifetime);
                quote! { & #lifetime #mutability #ty }
            }
            RustTy::Slice(ty, mutability, len) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };
                let len = len.as_ref().map(|len| {
                    let len = len.tokens(translator);
                    quote! { ; #len}
                });
                let ty = ty.tokens(translator, lifetime);
                quote! { & #lifetime #mutability [#ty #len] }
            }
            RustTy::CStr => quote! { core::ffi::CStr },
        }
    }
}

impl CExprItem {
    pub fn to_rust(
        items: impl Iterator<Item = impl Borrow<CExprItem>>,
        translator: &impl RustTokens,
    ) -> TokenStream {
        let mut output = TokenStream::new();
        let mut tmp_s = String::new();
        fn move_into_tokens(ts: &mut TokenStream, s: &mut String) {
            ts.extend::<TokenStream>(mem::take(s).parse().unwrap())
        }

        for item in items {
            match item.borrow() {
                CExprItem::Punct('~') => tmp_s.push('!'),
                CExprItem::Punct(c) => tmp_s.push(*c),
                CExprItem::NumericLiteral(lit) => tmp_s.push_str(lit),
                CExprItem::U32ArgVar(name) => tmp_s.push_str(name),
                CExprItem::StringLiteral(content) => tmp_s.push_str(&format!("c\"{content}\"")),
                CExprItem::MacroCall { macro_name, args } => {
                    move_into_tokens(&mut output, &mut tmp_s);
                    let name = translator.cmacro_tokens(*macro_name, true);

                    output.extend(if args.is_empty() {
                        quote! { #name }
                    } else {
                        let args = args
                            .iter()
                            .map(|arg| CExprItem::to_rust(arg.iter(), translator));

                        quote! { #name( #(#args),* ) }
                    });
                }
            }
        }

        move_into_tokens(&mut output, &mut tmp_s);
        output
    }
}
