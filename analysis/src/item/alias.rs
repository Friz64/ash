use crate::{item::RequiredBy, name::TypeName, xml};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Alias {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub alias: TypeName,
}

impl Alias {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::TypeAlias) -> Alias {
        trace!("constructing");
        Alias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        }
    }
}
