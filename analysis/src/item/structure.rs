use crate::{
    decl::{self, Decl},
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl ItemInfo for Struct {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> TypeName {
        self.name
    }
}

impl Struct {
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Structure,
    ) -> Struct {
        Struct {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(decl_ctx, &member.c_decl))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct Union {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl ItemInfo for Union {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> TypeName {
        self.name
    }
}

impl Union {
    pub(crate) fn new(
        decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Structure,
    ) -> Union {
        Union {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(decl_ctx, &member.c_decl))
                .collect(),
        }
    }
}
