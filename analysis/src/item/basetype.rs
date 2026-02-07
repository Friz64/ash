use crate::{
    item::{Named, RequireMap, RequiredBy},
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
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::BaseType) -> Option<BaseType> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(BaseType {
            required_by,
            name: xml.name,
        })
    }
}
