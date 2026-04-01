use super::{Code, Context};
use crate::{CodeMap, output::Destination};
use analysis::{
    item::{
        Named,
        alias::{CommandAlias, TypeAlias},
    },
    lifetime::Lifetime,
    to_rust::RustTranslator,
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for TypeAlias {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let name = ctx.type_to_rust(self.name(), false, &lifetime);
        let alias = ctx.type_to_rust(self.alias, true, &lifetime);

        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for CommandAlias {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.command_to_rust(self.name(), false);
        let alias = ctx.command_to_rust(self.alias, true);
        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
