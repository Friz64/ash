use crate::{item::RequiredBy, name::TypeName, xml};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl BitMask {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::BitMask) -> BitMask {
        trace!("constructing");
        BitMask {
            required_by,
            name: xml.name,
        }
    }
}

#[derive(Debug)]
pub struct BitMaskBits {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl BitMaskBits {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::BitMaskBits) -> BitMaskBits {
        trace!("constructing");
        BitMaskBits {
            required_by,
            name: xml.name,
        }
    }
}
