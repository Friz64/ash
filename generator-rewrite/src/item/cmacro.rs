use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::cmacro::CMacro, xml::cexpr::CExprItem};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for CMacro {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");

        let expr = CExprItem::to_rust(self.cexpr.iter(), ctx);
        let code = if self.args.is_empty() {
            let name = format_ident!("{}", self.name.prefix_trimmed());
            quote! {
                pub const #name: u32 = #expr;
            }
        } else {
            let name = format_ident!(
                "{}",
                self.name
                    .prefix_trimmed()
                    // todo: switch to future RustTranslate function?
                    .to_lowercase()
            );
            let args = self.args.iter().map(|arg| format_ident!("{arg}"));
            quote! {
                pub const fn #name(#(#args: u32),*) -> u32 { #expr }
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
