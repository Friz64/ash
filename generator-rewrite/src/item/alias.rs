use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::alias::Alias, to_rust::RustTranslator};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Alias {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let alias = ctx.type_to_rust(self.alias, true);
        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
