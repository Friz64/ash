use crate::{
    item::{Named, RequireMap, RequiredBy},
    xml::{self, name::TypeName},
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
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::TypeAlias) -> Option<Alias> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Alias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        })
    }
}
