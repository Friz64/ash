use crate::{
    decl::{self, Ty},
    item::RequiredBy,
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct FuncPointer {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub func_ty: Ty,
}

impl FuncPointer {
    #[instrument(skip(decl_ctx))]
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::FuncPointer,
    ) -> FuncPointer {
        trace!("constructing");
        FuncPointer {
            required_by,
            name: TypeName(xml.c_decl.name),
            func_ty: Ty::from_c(decl_ctx, &xml.c_decl.ty),
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub func_ty: Ty,
}

impl Command {
    #[instrument(skip(decl_ctx))]
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::FuncPointer,
    ) -> FuncPointer {
        trace!("constructing");
        FuncPointer {
            required_by,
            name: TypeName(xml.c_decl.name),
            func_ty: Ty::from_c(decl_ctx, &xml.c_decl.ty),
        }
    }
}
