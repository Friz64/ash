use crate::{
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl ItemInfo for Enum {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

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
