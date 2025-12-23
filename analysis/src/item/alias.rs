use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct Alias {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub alias: TypeName,
}

impl Item for Alias {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for Alias {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Alias {
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::TypeAlias,
    ) -> Alias {
        Alias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        }
    }
}
