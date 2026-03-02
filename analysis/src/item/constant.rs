use crate::{
    decl::CPrimaryType,
    item::{Named, RequireMap, RequiredBy},
    xml::{
        self,
        cexpr::{CExprItem, CExprItems},
        name::ConstantName,
    },
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub enum ConstantType {
    Integer(CPrimaryType),
    String,
}

#[derive(Debug)]
pub struct Constant {
    pub required_by: RequiredBy,
    pub name: ConstantName,
    pub ty: ConstantType,
    pub value: CExprItems,
}

impl Named<ConstantName> for Constant {
    fn name(&self) -> ConstantName {
        self.name
    }
}

impl Constant {
    #[instrument(skip(require_map))]
    pub(crate) fn from_base_constant(
        require_map: &RequireMap,
        xml: &xml::BaseConstant,
    ) -> Option<Constant> {
        let required_by = *require_map.constant.get(&xml.name)?;
        trace!(?required_by, "constructing from base constant");

        Some(Constant {
            required_by,
            name: xml.name,
            ty: ConstantType::Integer(CPrimaryType::from_str(xml.ty).unwrap()),
            value: xml.value.clone(),
        })
    }

    #[instrument]
    pub(crate) fn from_require(
        required_by: RequiredBy,
        xml: &xml::RequireConstant,
    ) -> Option<Constant> {
        trace!(?required_by, "constructing from require constant");

        let Some(value) = &xml.value else {
            return None;
        };

        Some(Constant {
            required_by,
            name: xml.name,
            ty: match value.as_slice() {
                [CExprItem::StringLiteral(..)] => ConstantType::String,
                _ => ConstantType::Integer(CPrimaryType::UInt32),
            },
            value: value.clone(),
        })
    }
}
