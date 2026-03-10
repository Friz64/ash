use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::basetype::BaseType, to_rust::RustTranslator};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let ty = self.ty.to_rust(ctx);
        let code = quote! {
            #[repr(transparent)]
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) #ty);
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
