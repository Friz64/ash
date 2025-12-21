use crate::{
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct BaseType {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl ItemInfo for BaseType {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> TypeName {
        self.name
    }
}

impl BaseType {
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::BaseType,
    ) -> BaseType {
        BaseType {
            required_by,
            name: xml.name,
        }
    }
}
