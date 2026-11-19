use super::{Code, Context};
use crate::{
    CodeMap,
    output::{GlobExport, OutputLocation},
};
use analysis::{
    item::alias::{CommandAlias, TypeAlias},
    rust::{Lifetime, RustTokens},
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for TypeAlias {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_of(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let lifetime = Lifetime(format_ident!("a"));
        let doc = crate::ref_page_doc_comment(self.name.original(), "Type alias");
        let name = octx.type_tokens(self.name, false, Some(&lifetime));
        let target = octx.type_tokens(self.target, true, Some(&lifetime));

        octx.emit_tokens(quote! {
            #[doc = #doc]
            pub type #name = #target;
        });
    }
}

impl Code for CommandAlias {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_of(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let doc = crate::ref_page_doc_comment(self.name.original(), "Command alias");
        let name = octx.command_tokens(self.name, false);
        let target = octx.command_tokens(self.target, true);

        octx.emit_tokens(quote! {
            #[doc = #doc]
            pub type #name = #target;
        });
    }
}
