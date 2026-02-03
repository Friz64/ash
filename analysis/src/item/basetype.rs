use crate::{
    item::{Named, RequiredBy, TypeRequireMap},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BaseType {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for BaseType {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl BaseType {
    #[instrument]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::BaseType) -> Option<BaseType> {
        let required_by = *trm.get(&xml.name)?;

        trace!("constructing");
        Some(BaseType {
            required_by,
            name: xml.name,
        })
    }
}
