use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::{EnumeratorName, TypeName},
    xml::{self, cexpr::CExprItems},
};
use indexmap::IndexMap;
use std::collections::HashMap;
use tracing::{instrument, trace};

#[derive(Debug, Clone, Copy)]
pub enum BitWidth {
    Bits32,
    Bits64,
}

#[derive(Debug)]
pub enum Value {
    BitPos(u8),
    Expr(CExprItems),
    Alias(EnumeratorName),
}

#[derive(Debug)]
pub struct Item {
    pub required_by: RequiredBy,
    pub value: Value,
}

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub bitmask_name: TypeName,
    pub bits_name: Option<TypeName>,
    pub bitwidth: BitWidth,
    pub items: IndexMap<EnumeratorName, Item>,
}

impl Named<TypeName> for BitMask {
    fn name(&self) -> TypeName {
        self.bitmask_name
    }
}

impl BitMask {
    #[instrument(skip(require_map))]
    pub(crate) fn new(
        require_map: &RequireMap,
        bitmask_bits_map: &HashMap<TypeName, &xml::BitMaskBits>,
        xml: &xml::BitMask,
    ) -> Option<BitMask> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        let bitwidth = match xml.ty.original() {
            "VkFlags64" => BitWidth::Bits64,
            "VkFlags" => BitWidth::Bits32,
            _ => unimplemented!(),
        };

        let bits_xml = (xml.bitvalues.or(xml.requires)).map(|name| bitmask_bits_map[&name]);
        let mut bitmask = BitMask {
            required_by,
            bitmask_name: xml.name,
            bits_name: bits_xml.map(|xml| xml.name),
            bitwidth,
            items: IndexMap::new(),
        };

        if let Some(bits_xml) = bits_xml {
            bitmask.extend(
                required_by,
                bits_xml.enumerators.iter().map(|en| (en.name, &en.value)),
            );
        }

        Some(bitmask)
    }

    pub(crate) fn extend<'a>(
        &mut self,
        required_by: RequiredBy,
        enumerators: impl Iterator<Item = (EnumeratorName, &'a xml::EnumeratorValue)>,
    ) {
        for (name, value) in enumerators {
            let value = match value {
                xml::EnumeratorValue::Expr(cexpr_items) => Value::Expr(cexpr_items.clone()),
                xml::EnumeratorValue::BitPos(bitpos) => Value::BitPos(*bitpos),
                xml::EnumeratorValue::Alias(name) => Value::Alias(*name),
                xml::EnumeratorValue::EnumOffset { .. } => unreachable!(),
            };

            let item = (self.items.entry(name)).or_insert_with(|| Item { required_by, value });
            item.required_by.merge(required_by);
        }
    }
}
