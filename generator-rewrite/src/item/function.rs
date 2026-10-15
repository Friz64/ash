use super::{Code, Context};
use crate::output::{CodeMap, GlobExport, OutputLocation};
use analysis::{
    item::function::{Command, FuncPointer},
    rust::RustTokens,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for FuncPointer {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);

        let name = octx.func_pointer_tokens(self.name, false);
        let params = self
            .params
            .iter()
            .map(|decl| decl.to_rust().tokens(&octx, None));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust().tokens(&octx, None);
            quote! { -> #rust_ty }
        });

        octx.insert_tokens(
            quote! { pub type #name = Option<unsafe extern "system" fn(#( #params ),*) #ret>; },
        );
    }
}

impl Code for Command {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);

        let name = octx.command_tokens(self.name, false);
        let params = self
            .params
            .iter()
            .map(|param| param.decl.to_rust().tokens(&octx, None));
        let ret = self.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust().tokens(&octx, None);
            quote! { -> #rust_ty }
        });

        octx.insert_tokens(
            quote! { pub type #name = unsafe extern "system" fn(#( #params ),*) #ret; },
        );
    }
}
