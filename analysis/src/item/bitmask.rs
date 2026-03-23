use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::{EnumVariantName, TypeName},
    xml::{self, cexpr::CExprItems},
};
use heck::ToShoutySnekCase;
use std::collections::HashMap;
use tracing::{instrument, trace};

#[derive(Debug)]
pub enum Value {
    BitPos(u8),
    Expr(CExprItems),
}

#[derive(Debug)]
pub struct Item {
    pub name: EnumVariantName,
    pub value: Value,
}

impl Item {
    pub fn stripped_name(&self, bits_name: TypeName) -> &'static str {
        let mut prefix = bits_name
            .tag_trimmed()
            .replace("FlagBits", "")
            .TO_SHOUTY_SNEK_CASE();

        // add _ before trailing number
        if prefix.ends_with(|c: char| c.is_ascii_digit()) {
            prefix.insert(prefix.len() - 1, '_');
        }

        prefix.push('_');
        self.name.original().strip_prefix(&prefix).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BitWidth {
    Bits32,
    Bits64,
}

#[derive(Debug)]
pub struct BitMask {
    pub required_by: RequiredBy,
    pub bitmask_name: TypeName,
    pub bits_name: Option<TypeName>,
    pub bitwidth: BitWidth,
    pub items: Vec<Item>,
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

        let mut items = Vec::new();

        let bits_xml = (xml.bitvalues.or(xml.requires)).map(|name| bitmask_bits_map[&name]);
        if let Some(bits_xml) = bits_xml {
            items.extend(bits_xml.values.iter().map(|value| match value {
                xml::BitMaskBitsItem::Bit(bit_mask_bit) => todo!(),
                xml::BitMaskBitsItem::Value(enum_value) => todo!(),
                xml::BitMaskBitsItem::Alias(enum_variant_alias) => todo!(),
            }));
        }

        Some(BitMask {
            required_by,
            bitmask_name: xml.name,
            bits_name: bits_xml.map(|xml| xml.name),
            bitwidth,
            items,
        })
    }

    pub(crate) fn extend(&mut self, extends: &xml::RequireEnumExtends) {
        match &extends.value {
            xml::RequireEnumExtendsValue::Value(_) => todo!(),
            xml::RequireEnumExtendsValue::BitPos(_) => todo!(),
            xml::RequireEnumExtendsValue::Alias(constant_name) => todo!(),
            xml::RequireEnumExtendsValue::Offset(require_enum_extends_offset) => todo!(),
        }
    }
}
