use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::alias::Alias, to_rust::RustTranslator};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Alias {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let alias = ctx.type_to_rust(self.alias);
        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
