use crate::{item::RequiredBy, name::TypeName, xml};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BaseType {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl BaseType {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::BaseType) -> BaseType {
        trace!("constructing");
        BaseType {
            required_by,
            name: xml.name,
        }
    }
}
