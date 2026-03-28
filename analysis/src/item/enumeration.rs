use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

// #[derive(Debug)]
// pub struct RequireEnumExtendsOffset {
//     pub offset: u32,
//     pub extension_num: u32,
//     pub positive_dir: bool,
// }

// impl RequireEnumExtendsOffset {
//     pub fn resolve_value(&self) -> i32 {
//         let ext_base = 1_000_000_000;
//         let ext_block_size = 1000;
//         let value = ext_base + (self.extension_num - 1) * ext_block_size + self.offset;

//         if self.positive_dir {
//             value as i32
//         } else {
//             -(value as i32)
//         }
//     }
// }

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Named<TypeName> for Enum {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Enum {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Enum) -> Option<Enum> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Enum {
            required_by,
            name: xml.name,
        })
    }
}
