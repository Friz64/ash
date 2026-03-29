use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::{EnumeratorName, TypeName},
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Handle {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub object_type: EnumeratorName,
    pub dispatchable: bool,
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
            object_type: xml.objtypeenum,
            dispatchable: match xml.ty {
                "VK_DEFINE_NON_DISPATCHABLE_HANDLE" => false,
                "VK_DEFINE_HANDLE" => true,
                unimplemented => unimplemented!("{unimplemented}"),
            },
        })
    }
}
