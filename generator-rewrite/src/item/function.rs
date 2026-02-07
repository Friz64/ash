use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::function::FuncPointer;
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.original());
        let code = quote! {
            #[allow(non_camel_case_types)]
            pub type #name = Option<()>;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
