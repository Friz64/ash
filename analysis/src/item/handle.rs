use crate::{item::RequiredBy, name::TypeName, xml};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Handle {
    pub required_by: RequiredBy,
    pub name: TypeName,
}

impl Handle {
    #[instrument]
    pub(crate) fn new(required_by: RequiredBy, xml: &xml::Handle) -> Handle {
        trace!("constructing");
        Handle {
            required_by,
            name: xml.name,
        }
    }
}
