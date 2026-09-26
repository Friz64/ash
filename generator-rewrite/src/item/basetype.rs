use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::basetype::BaseType, rust::RustTokens};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_tokens(self.name, false, None);
        let ty = self.ty.to_rust().tokens(ctx, None);
        let code = quote! {
            pub type #name = #ty;
        };

        CodeMap::new(Destination::primary_location(self.required_by), code)
    }
}
