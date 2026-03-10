use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::cmacro::CMacro, to_rust::RustTranslator, xml::cexpr::CExprItem};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for CMacro {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");

        let name = ctx.rust_name_of_cmacro(self);
        let expr = CExprItem::to_rust(self.cexpr.iter(), ctx);
        let code = if self.has_args() {
            quote! {
                pub const #name: u32 = #expr;
            }
        } else {
            let args = self.args.iter().map(|arg| format_ident!("{arg}"));
            quote! {
                pub const fn #name(#(#args: u32),*) -> u32 { #expr }
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
