use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{
        Named,
        constant::{Constant, ConstantType},
    },
    rust::RustTokens,
    xml::cexpr::CExprItem,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Constant {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.constant_tokens(self.name(), false);

        let ty = match self.ty {
            ConstantType::Integer(primary_ty) => ctx.primary_type_tokens(primary_ty),
            ConstantType::String => quote! { &core::ffi::CStr },
        };

        let value = CExprItem::to_rust(self.value.iter(), ctx);

        let code = quote! {
            pub const #name: #ty = #value;
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
