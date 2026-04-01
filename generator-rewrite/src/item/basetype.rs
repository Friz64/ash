use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{Named, basetype::BaseType},
    lifetime::Lifetime,
    to_rust::RustTranslator,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_to_rust(self.name(), false, &Lifetime::placeholder());
        let ty = self.ty.to_rust(ctx, &Lifetime::placeholder());
        let code = quote! {
            pub type #name = #ty;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
