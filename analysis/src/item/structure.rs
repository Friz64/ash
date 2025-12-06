use crate::xml;

#[derive(Debug)]
pub struct Structure {
    pub name: &'static str,
}

impl Structure {
    pub fn new(xml: &xml::Structure) -> Structure {
        Structure { name: xml.name }
    }
}
