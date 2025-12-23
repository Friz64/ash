use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::basetype::BaseType;
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument]
    fn code(&self, _ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let code = quote! {
            #[repr(transparent)]
            #[allow(non_camel_case_types)]
            pub struct #name(pub(crate) i16);
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
