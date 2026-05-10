use proc_macro2::{Literal, TokenStream};

use crate::{
    item::{RequireMap, structure::Length},
    name::{ConstantName, FuncPointerName, TypeName, VariableName},
    to_rust::RustTranslator,
    xml::{
        cdecl::{CArrayLen, CDecl, CType},
        cexpr::{self, CExprItem, CExprItems},
    },
};

#[derive(Debug)]
pub struct Decl {
    pub name: VariableName,
    pub ty: Ty,
}

impl Decl {
    pub(crate) fn from_c(
        require_map: &RequireMap,
        c_decl: &CDecl<'static>,
    ) -> Decl {
        Decl {
            name: VariableName::new(c_decl.name),
            ty: Ty::from_c(require_map, &c_decl.ty),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Not,
    Mut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CPrimaryType {
    Void,
    Char,
    Int,
    Float,
    Double,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Size,
}

impl CPrimaryType {
    pub(crate) fn from_str(s: &str) -> Option<CPrimaryType> {
        match s {
            "void" => Some(CPrimaryType::Void),
            "char" => Some(CPrimaryType::Char),
            "int" => Some(CPrimaryType::Int),
            "float" => Some(CPrimaryType::Float),
            "double" => Some(CPrimaryType::Double),
            "int8_t" => Some(CPrimaryType::Int8),
            "uint8_t" => Some(CPrimaryType::UInt8),
            "int16_t" => Some(CPrimaryType::Int16),
            "uint16_t" => Some(CPrimaryType::UInt16),
            "int32_t" => Some(CPrimaryType::Int32),
            "uint32_t" => Some(CPrimaryType::UInt32),
            "int64_t" => Some(CPrimaryType::Int64),
            "uint64_t" => Some(CPrimaryType::UInt64),
            "size_t" => Some(CPrimaryType::Size),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArrayLen {
    Constant(ConstantName),
    Literal(u128),
}

impl ArrayLen {
    pub fn to_rust(&self, translator: &impl RustTranslator) -> TokenStream {
        match self {
            ArrayLen::Constant(constant_name) => translator.constant_to_rust(*constant_name, true),
            ArrayLen::Literal(value) => {
                let literal = Literal::u128_unsuffixed(*value);
                quote::quote! { #literal }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Ty {
    SpecType(TypeName),
    SpecFuncPointer(FuncPointerName),
    CPrimary(CPrimaryType),
    RustType(RustType),
    Platform(&'static str),
    Ptr(&'static Ty, Mutability),
    Ref(&'static Ty, Mutability),
    Slice(&'static Ty, Mutability, Option<ArrayLen>),
    Array(&'static Ty, ArrayLen),
}

#[derive(Debug, Clone, Copy)]
pub enum RustType {
    CStr,
}

impl Ty {
    pub(crate) fn from_c(
        require_map: &RequireMap,
        c_type: &CType<'static>,
    ) -> Ty {
        match c_type {
            CType::Base(cbase_type) => {
                let name = cbase_type.name;
                if let Some(primary) = CPrimaryType::from_str(name) {
                    Ty::CPrimary(primary)
                } else if require_map.ty.contains_key(&TypeName::new(name)) {
                    Ty::SpecType(TypeName::new(name))
                } else if require_map
                    .func_pointer
                    .contains_key(&FuncPointerName::new(name))
                {
                    Ty::SpecFuncPointer(FuncPointerName::new(name))
                } else {
                    Ty::Platform(name)
                }
            }
            CType::Ptr {
                implicit_for_decay: _,
                is_const,
                pointee,
            } => Ty::Ptr(
                Box::leak(Box::new(Ty::from_c(require_map, pointee))),
                if *is_const {
                    Mutability::Not
                } else {
                    Mutability::Mut
                },
            ),
            CType::Array { element, len } => Ty::Array(
                Box::leak(Box::new(Ty::from_c(require_map, element))),
                match len {
                    CArrayLen::Named(constant) => ArrayLen::Constant(ConstantName::new(constant)),
                    CArrayLen::Literal(value) => ArrayLen::Literal(*value),
                },
            ),
            CType::Func { .. } => unreachable!("unused after Vulkan-Headers 339"),
        }
    }
}
