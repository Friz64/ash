use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Item for BitMask {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for BitMask {
    fn name(&self) -> TypeName {
        self.name
    }
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

impl Item for BitMaskBits {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Type for BitMaskBits {
    fn name(&self) -> TypeName {
        self.name
    }
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
