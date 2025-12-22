use crate::{
    cdecl::{CDecl, CType},
    item::RequiredBy,
    name::TypeName,
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
pub enum CBaseTy {
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

#[derive(Debug)]
pub struct ArrayLen;

#[derive(Debug)]
pub enum Ty {
    Spec(TypeName),
    CBase(CBaseTy),
    External(&'static str),
    Ptr(&'static Ty, Mutability),
    Ref(&'static Ty, Mutability),
    Array(&'static Ty, ArrayLen /* todo */),
    Func {
        ret_ty: Option<&'static Ty>,
        params: Vec<&'static Decl>,
    },
}

impl Ty {
    pub(crate) fn from_c(ctx: &Context, c_type: &CType<'static>) -> Ty {
        match c_type {
            CType::Base(cbase_type) => match cbase_type.name {
                "void" => Ty::CBase(CBaseTy::Void),
                "char" => Ty::CBase(CBaseTy::Char),
                "int" => Ty::CBase(CBaseTy::Int),
                "float" => Ty::CBase(CBaseTy::Float),
                "double" => Ty::CBase(CBaseTy::Double),
                "int8_t" => Ty::CBase(CBaseTy::Int8),
                "uint8_t" => Ty::CBase(CBaseTy::UInt8),
                "int16_t" => Ty::CBase(CBaseTy::Int16),
                "uint16_t" => Ty::CBase(CBaseTy::UInt16),
                "int32_t" => Ty::CBase(CBaseTy::Int32),
                "uint32_t" => Ty::CBase(CBaseTy::UInt32),
                "int64_t" => Ty::CBase(CBaseTy::Int64),
                "uint64_t" => Ty::CBase(CBaseTy::UInt64),
                "size_t" => Ty::CBase(CBaseTy::Size),
                spec if ctx.type_require_map.contains_key(&TypeName(spec)) => {
                    Ty::Spec(TypeName(spec))
                }
                external => Ty::External(external),
            },
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
            CType::Array {
                element,
                len: _todo,
            } => Ty::Array(Box::leak(Box::new(Ty::from_c(ctx, element))), ArrayLen),
            CType::Func { ret_ty, params } => Ty::Func {
                ret_ty: ret_ty
                    .as_ref()
                    .map(|c_type| &*Box::leak(Box::new(Ty::from_c(ctx, c_type)))),
                params: params
                    .iter()
                    .map(|c_decl| &*Box::leak(Box::new(Decl::from_c(ctx, c_decl))))
                    .collect(),
            },
        }
    }
}
