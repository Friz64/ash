use crate::{
    decl::CPrimaryType,
    item::{Item, RequiredBy},
    name::ConstantName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Expression(pub &'static str);

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

impl Item for Constant {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }
}

impl Constant {
    #[instrument]
    pub(crate) fn from_constant(required_by: RequiredBy, xml: &xml::Constant) -> Constant {
        trace!("constructing from constant");

        Constant {
            required_by,
            name: xml.name,
            value: Value::Expression(Some(xml.ty), Expression(xml.value)),
        }
    }

    #[instrument]
    pub(crate) fn from_require_constant(
        required_by: RequiredBy,
        xml: &xml::RequireConstant,
    ) -> Option<Constant> {
        trace!("constructing from require constant");

        let value = xml.value?;
        Some(Constant {
            required_by,
            name: xml.name,
            value: if let Some(string) = value.strip_prefix('"') {
                Value::LiteralString(string.strip_suffix('"').unwrap())
            } else {
                Value::Expression(None, Expression(value))
            },
        })
    }
}
