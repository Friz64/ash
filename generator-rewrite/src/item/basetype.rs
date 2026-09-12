use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::basetype::BaseType, lifetime::Lifetime, rust::RustTokens};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_tokens(self.name, false, &Lifetime::placeholder());
        let ty = self.ty.to_rust().tokens(ctx, &Lifetime::placeholder());
        let code = quote! {
            pub type #name = #ty;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
