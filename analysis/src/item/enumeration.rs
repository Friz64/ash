use crate::{
    item::{Named, RequiredBy, TypeRequireMap},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for Enum {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Enum {
    #[instrument]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::Enum) -> Option<Enum> {
        let name = xml.name;
        let required_by = *trm.get(&name)?;

        trace!("constructing");
        Some(Enum {
            required_by,
            name: xml.name,
        })
    }
}
