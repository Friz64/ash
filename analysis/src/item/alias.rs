use crate::{
    item::{Named, RequiredBy, TypeRequireMap},
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

impl Named<TypeName> for Alias {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Alias {
    #[instrument]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::TypeAlias) -> Option<Alias> {
        let required_by = *trm.get(&xml.name)?;

        trace!("constructing");
        Some(Alias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        })
    }
}
