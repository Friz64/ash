use crate::{
    decl::CPrimaryType,
    item::{ConstantRequireMap, Named, RequiredBy},
    name::ConstantName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Expression(pub &'static str);

// does this is sense????? ?
#[derive(Debug)]
pub enum Value {
    LiteralString(&'static str),
    Expression(Option<CPrimaryType>, Expression),
}

#[derive(Debug)]
pub struct Constant {
    pub required_by: RequiredBy,
    pub name: ConstantName,
    pub value: Value,
}

impl Named<ConstantName> for Constant {
    fn name(&self) -> ConstantName {
        self.name
    }
}

impl Constant {
    #[instrument(skip(crm))]
    pub(crate) fn from_base_constant(
        crm: &ConstantRequireMap,
        xml: &xml::BaseConstant,
    ) -> Option<Constant> {
        let required_by = *crm.get(&xml.name)?;
        trace!(?required_by, "constructing from constant");

        Some(Constant {
            required_by,
            name: xml.name,
            value: Value::Expression(Some(xml.ty), Expression(xml.value)),
        })
    }

    #[instrument]
    pub(crate) fn from_require_constant(
        required_by: RequiredBy,
        xml: &xml::RequireConstant,
    ) -> Option<Constant> {
        let xml_value = xml.value?;
        trace!("constructing from require constant");

        let value = if let Some(string) = xml_value.strip_prefix('"') {
            Value::LiteralString(string.strip_suffix('"').unwrap())
        } else {
            Value::Expression(None, Expression(xml_value))
        };

        Some(Constant {
            required_by,
            name: xml.name,
            value,
        })
    }
}
