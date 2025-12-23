use crate::{
    item::{Item, RequiredBy, Type},
    name::TypeName,
    xml,
};

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
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::BitMask,
    ) -> BitMask {
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
    pub(crate) fn new(
        // decl_ctx: &decl::Context,
        required_by: RequiredBy,
        xml: &xml::BitMaskBits,
    ) -> BitMaskBits {
        BitMaskBits {
            required_by,
            name: xml.name,
        }
    }
}
