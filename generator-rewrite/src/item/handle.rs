use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::handle::Handle, to_rust::RustName};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Handle {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) i32);
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
