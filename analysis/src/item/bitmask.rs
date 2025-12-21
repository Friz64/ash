use crate::{
    item::{ItemInfo, RequiredBy},
    name::TypeName,
    xml,
};

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl ItemInfo for BitMask {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

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

impl ItemInfo for BitMaskBits {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

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
