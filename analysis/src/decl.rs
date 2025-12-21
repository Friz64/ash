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
