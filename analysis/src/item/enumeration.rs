use crate::{
    item::{Named, RequireMap, RequiredBy},
    xml::{self, name::TypeName},
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
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Enum) -> Option<Enum> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Enum {
            required_by,
            name: xml.name,
        })
    }
}
