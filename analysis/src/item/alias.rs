use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

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
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::TypeAlias) -> Alias {
        trace!("constructing");
        Alias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        }
    }
}
