use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::funcpointer::FuncPointer;
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let fnptr = self.func_ty.to_rust(ctx);
        let code = quote! {
            #[allow(non_camel_case_types)]
            pub type #name = Option<#fnptr>;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
