use crate::{
    item::{Named, RequireMap, RequiredBy},
    xml::{self, name::TypeName},
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Handle {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for Handle {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Handle {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Handle) -> Option<Handle> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Handle {
            required_by,
            name: xml.name,
        })
    }
}
