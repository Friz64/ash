use crate::{
    decl::{self, Ty},
    item::{Item, RequiredBy, Type},
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

impl Item for FuncPointer {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for FuncPointer {
    fn name(&self) -> TypeName {
        self.name
    }
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
