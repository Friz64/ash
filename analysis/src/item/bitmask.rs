use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for BitMask {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl BitMask {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::BitMask) -> Option<BitMask> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(BitMask {
            required_by,
            name: xml.name,
        })
    }
}

#[derive(Debug)]
pub struct BitMaskBits {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for BitMaskBits {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl BitMaskBits {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::BitMaskBits) -> Option<BitMaskBits> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(BitMaskBits {
            required_by,
            name: xml.name,
        })
    }
}
