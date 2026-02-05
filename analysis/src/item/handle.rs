use crate::{
    item::{Named, RequiredBy, TypeRequireMap},
    name::TypeName,
    xml,
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
    #[instrument(skip(trm))]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::Handle) -> Option<Handle> {
        let required_by = *trm.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Handle {
            required_by,
            name: xml.name,
        })
    }
}
