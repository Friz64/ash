use crate::{
    cdecl::{CDecl, CType},
    item::{EmergeCtx, Item},
};

#[derive(Debug)]
pub struct Decl {
    pub name: &'static str,
    pub ty: Ty,
}

impl Decl {
    pub(crate) fn from_c(emerge_ctx: &mut EmergeCtx, c_decl: &CDecl<'static>) -> Decl {
        Decl {
            name: c_decl.name,
            ty: Ty::from_c(emerge_ctx, &c_decl.ty),
        }
    }
}

#[derive(Debug)]
pub enum Mutability {
    Not,
    Mut,
}

#[derive(Debug)]
pub enum BaseTy {
    Void,
}

#[derive(Debug)]
pub struct ArrayLen;

#[derive(Debug)]
pub enum Ty {
    Item(&'static Item),
    Base(BaseTy),
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
    pub(crate) fn from_c(emerge_ctx: &mut EmergeCtx, c_type: &CType<'static>) -> Ty {
        match c_type {
            CType::Base(cbase_type) => match cbase_type.name {
                "void" => Ty::Base(BaseTy::Void),
                other => {
                    if let Some(item) = Item::emerge(emerge_ctx, other) {
                        Ty::Item(item)
                    } else {
                        Ty::External(other)
                    }
                }
            },
            CType::Ptr {
                implicit_for_decay: _,
                is_const,
                pointee,
            } => Ty::Ptr(
                Box::leak(Box::new(Ty::from_c(emerge_ctx, pointee))),
                if *is_const {
                    Mutability::Not
                } else {
                    Mutability::Mut
                },
            ),
            CType::Array {
                element,
                len: _todo,
            } => Ty::Array(
                Box::leak(Box::new(Ty::from_c(emerge_ctx, element))),
                ArrayLen,
            ),
            CType::Func { ret_ty, params } => Ty::Func {
                ret_ty: ret_ty
                    .as_ref()
                    .map(|c_type| &*Box::leak(Box::new(Ty::from_c(emerge_ctx, c_type)))),
                params: params
                    .iter()
                    .map(|c_decl| &*Box::leak(Box::new(Decl::from_c(emerge_ctx, c_decl))))
                    .collect(),
            },
        }
    }
}
