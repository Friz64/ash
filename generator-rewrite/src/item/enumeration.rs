use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::enumeration::Enum;
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Enum {
    #[instrument]
    fn code(&self, _ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let code = quote! {
            #[repr(transparent)]
            pub struct #name(pub(crate) i32);
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
