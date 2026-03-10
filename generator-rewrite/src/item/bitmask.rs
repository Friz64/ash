use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::bitmask::{BitMask, BitMaskBits},
    to_rust::RustTranslator,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BitMask {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) i32);
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for BitMaskBits {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) i32);
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
