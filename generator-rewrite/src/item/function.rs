use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{
        Named,
        function::{Command, FuncPointer},
    },
    lifetime::Lifetime,
    to_rust::RustTranslator,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.func_pointer_to_rust(self.name(), false);
        let params = self
            .params
            .iter()
            .map(|decl| decl.to_rust(ctx, &Lifetime::placeholder()));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust(ctx, &Lifetime::placeholder());
            quote! { -> #rust_ty }
        });

        let code = quote! {
            pub type #name = Option<unsafe extern "system" fn(#( #params ),*) #ret>;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for Command {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.command_to_rust(self.name(), false);
        let params = self
            .params
            .iter()
            .map(|param| param.decl.to_rust(ctx, &Lifetime::placeholder()));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust(ctx, &Lifetime::placeholder());
            quote! { -> #rust_ty }
        });

        let code = quote! {
            pub type #name = unsafe extern "system" fn(#( #params ),*) #ret;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
