use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BaseType {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Item for BaseType {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for BaseType {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl BaseType {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::BaseType) -> BaseType {
        trace!("constructing");
        BaseType {
            required_by,
            name: xml.name,
        }
    }
}
