use super::{Code, Context};
use crate::output::{CodeMap, GlobExport, OutputLocation};
use analysis::{item::cmacro::CMacro, rust::RustTokens, xml::cexpr::CExprItem};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for CMacro {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");

        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let name = octx.cmacro_tokens(self.name, false);
        let expr = CExprItem::tokens(self.cexpr.iter(), &octx);
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

        octx.insert_tokens(code);
    }
}
