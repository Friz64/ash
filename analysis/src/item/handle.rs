use crate::{
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Handle {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl ItemInfo for Handle {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> TypeName {
        self.name
    }
}

impl Handle {
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::Handle,
    ) -> Handle {
        Handle {
            required_by,
            name: xml.name,
        }
    }
}
