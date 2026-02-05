use crate::{
    item::{Named, RequiredBy, TypeRequireMap},
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
    #[instrument(skip(trm))]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::BitMask) -> Option<BitMask> {
        let required_by = *trm.get(&xml.name)?;
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
    #[instrument(skip(trm))]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::BitMaskBits) -> Option<BitMaskBits> {
        let required_by = *trm.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(BitMaskBits {
            required_by,
            name: xml.name,
        })
    }
}
