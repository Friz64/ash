use crate::{
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Enumeration {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl ItemInfo for Enumeration {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> TypeName {
        self.name
    }
}

impl Enumeration {
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Enum,
    ) -> Enumeration {
        Enumeration {
            required_by,
            name: xml.name,
        }
    }
}
