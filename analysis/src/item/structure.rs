use crate::{
    decl::{self, Decl, Ty},
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl Item for Struct {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for Struct {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Struct {
    #[instrument(skip(decl_ctx))]
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Structure,
    ) -> Struct {
        trace!("constructing");
        Struct {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(decl_ctx, &member.c_decl))
                .collect(),
        }
    }

    pub fn has_pointer(&self) -> bool {
        has_pointer(&self.members)
    }
}

#[derive(Debug)]
pub struct Union {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl Item for Union {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for Union {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Union {
    #[instrument(skip(decl_ctx))]
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Structure,
    ) -> Union {
        trace!("constructing");
        Union {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(decl_ctx, &member.c_decl))
                .collect(),
        }
    }

    pub fn has_pointer(&self) -> bool {
        has_pointer(&self.members)
    }
}

fn has_pointer(members: &[Decl]) -> bool {
    members
        .iter()
        .any(|member| matches!(member.ty, Ty::Ptr(..)))
}
