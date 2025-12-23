use crate::{
    item::{Item, RequiredBy},
    name::ConstantName,
    xml,
};

#[derive(Debug)]
pub enum Value {
    String(&'static str),
    Todo,
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
    pub(crate) fn from_constant(required_by: RequiredBy, xml: &xml::Constant) -> Constant {
        Constant {
            required_by,
            name: xml.name,
            value: Value::Todo,
        }
    }

    pub(crate) fn from_require_constant(
        required_by: RequiredBy,
        xml: &xml::RequireConstant,
    ) -> Option<Constant> {
        let value = xml.value?;
        Some(Constant {
            required_by,
            name: xml.name,
            value: if let Some(string) = value.strip_prefix('"') {
                Value::String(string.strip_suffix('"').unwrap())
            } else {
                Value::Todo
            },
        })
    }
}
