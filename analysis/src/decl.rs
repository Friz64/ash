use crate::{
    item::RequiredBy,
    name::{ConstantName, TypeName},
    xml::cdecl::{CArrayLen, CDecl, CType},
};
use std::collections::HashMap;

pub struct Context<'a> {
    type_require_map: &'a HashMap<TypeName, RequiredBy>,
}

impl<'a> Context<'a> {
    pub fn new(type_require_map: &'a HashMap<TypeName, RequiredBy>) -> Self {
        Context { type_require_map }
    }
}

#[derive(Debug)]
pub struct Decl {
    pub name: &'static str,
    pub ty: Ty,
}

impl Decl {
    pub(crate) fn from_c(ctx: &Context, c_decl: &CDecl<'static>) -> Decl {
        Decl {
            name: c_decl.name,
            ty: Ty::from_c(ctx, &c_decl.ty),
        }
    }
}

#[derive(Debug)]
pub enum Mutability {
    Not,
    Mut,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug)]
pub enum ArrayLen {
    Constant(ConstantName),
    Literal(u128),
}

#[derive(Debug)]
pub enum Ty {
    Spec(TypeName),
    CPrimary(CPrimaryType),
    External(&'static str),
    Ptr(&'static Ty, Mutability),
    Ref(&'static Ty, Mutability),
    Array(&'static Ty, ArrayLen),
}

impl Ty {
    pub(crate) fn from_c(ctx: &Context, c_type: &CType<'static>) -> Ty {
        match c_type {
            CType::Base(cbase_type) => {
                let name = cbase_type.name;
                if let Some(primary) = CPrimaryType::from_str(name) {
                    Ty::CPrimary(primary)
                } else if ctx.type_require_map.contains_key(&TypeName(name)) {
                    Ty::Spec(TypeName(name))
                } else {
                    Ty::External(name)
                }
            }
            CType::Ptr {
                implicit_for_decay: _,
                is_const,
                pointee,
            } => Ty::Ptr(
                Box::leak(Box::new(Ty::from_c(ctx, pointee))),
                if *is_const {
                    Mutability::Not
                } else {
                    Mutability::Mut
                },
            ),
            CType::Array { element, len } => Ty::Array(
                Box::leak(Box::new(Ty::from_c(ctx, element))),
                match len {
                    CArrayLen::Named(constant) => ArrayLen::Constant(ConstantName(constant)),
                    CArrayLen::Literal(value) => ArrayLen::Literal(*value),
                },
            ),
            CType::Func { .. } => unreachable!("unused after Vulkan-Headers >339"),
        }
    }
}
