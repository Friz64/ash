use crate::{item::RequiredBy, name::TypeName, xml};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Enum {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Enum {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::Enum) -> Enum {
        trace!("constructing");
        Enum {
            required_by,
            name: xml.name,
        }
    }
}
