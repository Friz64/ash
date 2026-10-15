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
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let lifetime = Lifetime(format_ident!("a"));
        let name = octx.type_tokens(self.name, false, Some(&lifetime));
        let alias = octx.type_tokens(self.alias, true, Some(&lifetime));

        let code = quote! {
            pub type #name = #alias;
        };

        octx.insert_tokens(code)
    }
}

impl Code for CommandAlias {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let name = octx.command_tokens(self.name, false);
        let alias = octx.command_tokens(self.alias, true);
        let code = quote! {
            pub type #name = #alias;
        };

        octx.insert_tokens(code)
    }
}
