use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::function::FuncPointer, to_rust::RustName};
use quote::quote;
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let code = quote! {
            #[allow(non_camel_case_types)]
            pub type #name = Option<()>;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
