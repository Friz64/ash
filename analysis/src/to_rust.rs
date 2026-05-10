use crate::{
    decl::{CPrimaryType, Decl, Mutability, RustType, Ty},
    lifetime::Lifetime,
    name::{
        CMacroName, CommandName, ConstantName, EnumeratorName, FuncPointerName, TypeName,
        VariableName,
    },
    xml::cexpr::CExprItem,
};
use proc_macro2::{TokenStream};
use quote::quote;
use std::{borrow::Borrow, mem};
use syn::Ident;

pub trait RustTranslator {
    fn var_name_to_rust(&self, name: VariableName) -> Ident;

    fn type_to_rust(&self, name: TypeName, qualified: bool, lifetime: &Lifetime) -> TokenStream;

    fn func_pointer_to_rust(&self, name: FuncPointerName, qualified: bool) -> TokenStream;

    fn command_to_rust(&self, name: CommandName, qualified: bool) -> TokenStream;

    fn constant_to_rust(&self, name: ConstantName, qualified: bool) -> TokenStream;

    fn enumerator_to_rust(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        qualified: bool,
    ) -> TokenStream;

    fn cmacro_to_rust(&self, name: CMacroName, qualified: bool) -> TokenStream;

    fn platform_type_to_rust(&self, raw: &str, qualified: bool) -> TokenStream;

    fn rust_type_to_rust(&self, ty: &RustType) -> TokenStream;

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
}

impl Decl {
    /// Gives you this declaration in the form of `#name: #ty`.
    pub fn to_rust(&self, translator: &impl RustTranslator, lifetime: &Lifetime) -> TokenStream {
        let name = translator.var_name_to_rust(self.name);
        let ty = self.ty.to_rust(translator, lifetime);
        quote! { #name: #ty }
    }
}

impl Ty {
    pub fn to_rust(&self, translator: &impl RustTranslator, lifetime: &Lifetime) -> TokenStream {
        match self {
            Ty::SpecType(name) => translator.type_to_rust(*name, true, lifetime),
            Ty::SpecFuncPointer(name) => translator.func_pointer_to_rust(*name, true),
            Ty::CPrimary(base_ty) => translator.primary_type_to_rust(*base_ty),
            Ty::Platform(raw) => translator.platform_type_to_rust(raw, true),
            Ty::Ptr(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! { const },
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(translator, lifetime);
                quote! { * #mutability #ty }
            }
            Ty::Ref(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(translator, lifetime);
                quote! { & #lifetime #mutability #ty }
            }
            Ty::Slice(ty, mutability, len) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };
                let len = len.as_ref().map(|len| {
                    let len = len.to_rust(translator);
                    quote! { ; #len}
                });
                let ty = ty.to_rust(translator, lifetime);
                quote! { & #lifetime #mutability [#ty #len] }
            }
            Ty::Array(ty, array_len) => {
                let ty = ty.to_rust(translator, lifetime);
                let array_len = array_len.to_rust(translator);
                quote! { [#ty; #array_len as _] }
            }
            Ty::RustType(ty) => {
                let ty = translator.rust_type_to_rust(ty);
                quote! { #ty }
            }
        }
    }
}

impl CExprItem {
    pub fn to_rust(
        items: impl Iterator<Item = impl Borrow<CExprItem>>,
        translator: &impl RustTranslator,
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
                    let name = translator.cmacro_to_rust(*macro_name, true);

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
