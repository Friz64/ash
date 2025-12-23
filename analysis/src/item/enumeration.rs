use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Item for Enum {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for Enum {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Enum {
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Enum,
    ) -> Enum {
        Enum {
            required_by,
            name: xml.name,
        }
    }
}
