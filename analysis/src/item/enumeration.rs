use crate::{
    item::{Named, RequireMap, RequiredBy},
    name::{EnumeratorName, TypeName},
    xml::{self, cexpr::CExprItems},
};
use indexmap::IndexMap;
use tracing::{instrument, trace};

#[derive(Debug)]
pub enum Value {
    Variant(i32),
    Expr(CExprItems),
    Alias(EnumeratorName),
}

#[derive(Debug)]
pub struct Item {
    pub required_by: RequiredBy,
    pub value: Value,
}

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub items: IndexMap<EnumeratorName, Item>,
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

        let mut enumeration = Enum {
            required_by,
            name: xml.name,
            items: IndexMap::new(),
        };

        enumeration.extend(
            required_by,
            xml.enumerators.iter().map(|en| (en.name, &en.value)),
        );

        Some(enumeration)
    }

    pub(crate) fn extend<'a>(
        &mut self,
        required_by: RequiredBy,
        enumerators: impl Iterator<Item = (EnumeratorName, &'a xml::EnumeratorValue)>,
    ) {
        for (name, value) in enumerators {
            let value = match value {
                xml::EnumeratorValue::Expr(cexpr_items) => Value::Expr(cexpr_items.clone()),
                xml::EnumeratorValue::BitPos(..) => unreachable!(),
                xml::EnumeratorValue::Alias(name) => Value::Alias(*name),
                xml::EnumeratorValue::EnumOffset {
                    offset,
                    extension_num,
                    positive_dir,
                } => {
                    let ext_base = 1_000_000_000;
                    let ext_block_size = 1000;
                    let value = (ext_base + (extension_num - 1) * ext_block_size + offset) as i32;
                    Value::Variant(if *positive_dir { value } else { -value })
                }
            };

            let item = (self.items.entry(name)).or_insert_with(|| Item { required_by, value });
            item.required_by.merge(required_by);
        }
    }
}
