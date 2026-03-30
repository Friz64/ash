use super::{Code, Context};
use crate::{CodeMap, output::Destination};
use analysis::{
    item::alias::{CommandAlias, TypeAlias},
    to_rust::{RustName, RustTranslator},
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for TypeAlias {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let alias = ctx.type_to_rust(self.alias, true);
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
        let name = self.rust_name(ctx);
        let alias = ctx.command_to_rust(self.alias, true);
        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
