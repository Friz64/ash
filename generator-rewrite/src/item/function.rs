use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::function::{Command, FuncPointer},
    to_rust::RustName,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let params = self.params.iter().map(|decl| decl.to_rust(ctx));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust(ctx);
            quote! { -> #rust_ty }
        });

        let code = quote! {
            pub type #name = Option<unsafe extern "system" fn(#( #params ),*) #ret>;
        };

        CodeMap::new(Destination::library(self.required_by), code)
    }
}

impl Code for Command {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let params = self.params.iter().map(|param| param.decl.to_rust(ctx));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust(ctx);
            quote! { -> #rust_ty }
        });

        let code = quote! {
            pub type #name = unsafe extern "system" fn(#( #params ),*) #ret;
        };

        CodeMap::new(Destination::library(self.required_by), code)
    }
}
