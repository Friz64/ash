use crate::{
    decl::{ArrayLen, CPrimaryType, Decl, Mutability, Ty},
    item::Named,
    name::{CMacroName, ConstantName, EnumeratorName, FuncPointerName, TypeName, VariableName},
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::{borrow::Borrow, mem};
use syn::Ident;

pub trait RustTranslator {
    fn var_name_to_rust(&self, name: VariableName) -> Ident;

    fn type_to_rust(&self, name: TypeName, with_path: bool) -> TokenStream;

    fn func_pointer_to_rust(&self, name: FuncPointerName, with_path: bool) -> TokenStream;

    fn constant_to_rust(&self, name: ConstantName, with_path: bool) -> TokenStream;

    fn enumerator_to_rust(
        &self,
        name: EnumeratorName,
        type_name: TypeName,
        with_path: bool,
    ) -> TokenStream;

    fn cmacro_to_rust(&self, name: CMacroName, with_path: bool) -> TokenStream;

    fn platform_type_to_rust(&self, raw: &'static str, with_path: bool) -> TokenStream;

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

pub trait RustName<N> {
    fn rust_name(&self, rust_translator: &impl RustTranslator) -> TokenStream;
}

impl<T: Named<TypeName>> RustName<TypeName> for T {
    fn rust_name(&self, rust_translator: &impl RustTranslator) -> TokenStream {
        rust_translator.type_to_rust(self.name(), false)
    }
}

impl<T: Named<FuncPointerName>> RustName<FuncPointerName> for T {
    fn rust_name(&self, rust_translator: &impl RustTranslator) -> TokenStream {
        rust_translator.func_pointer_to_rust(self.name(), false)
    }
}

impl<T: Named<ConstantName>> RustName<ConstantName> for T {
    fn rust_name(&self, rust_translator: &impl RustTranslator) -> TokenStream {
        rust_translator.constant_to_rust(self.name(), false)
    }
}

impl<T: Named<CMacroName>> RustName<CMacroName> for T {
    fn rust_name(&self, rust_translator: &impl RustTranslator) -> TokenStream {
        rust_translator.cmacro_to_rust(self.name(), false)
    }
}

impl Decl {
    /// Gives you this declaration in the form of `#name: #ty`.
    pub fn to_rust(&self, translator: &impl RustTranslator) -> TokenStream {
        let name = translator.var_name_to_rust(self.name);
        let ty = self.ty.to_rust(translator);
        quote! { #name: #ty }
    }
}

impl Ty {
    pub fn to_rust(&self, translator: &impl RustTranslator) -> TokenStream {
        match self {
            Ty::SpecType(name) => translator.type_to_rust(*name, true),
            Ty::SpecFuncPointer(name) => translator.func_pointer_to_rust(*name, true),
            Ty::CPrimary(base_ty) => translator.primary_type_to_rust(*base_ty),
            Ty::Platform(raw) => translator.platform_type_to_rust(raw, true),
            Ty::Ptr(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! { const },
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(translator);
                quote! { * #mutability #ty }
            }
            Ty::Ref(ty, mutability) => {
                let mutability = match mutability {
                    Mutability::Not => quote! {},
                    Mutability::Mut => quote! { mut },
                };

                let ty = ty.to_rust(translator);
                quote! { & #mutability #ty }
            }
            Ty::Array(ty, array_len) => {
                let ty = ty.to_rust(translator);
                let array_len = match array_len {
                    ArrayLen::Constant(constant) => translator.constant_to_rust(*constant, true),
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
