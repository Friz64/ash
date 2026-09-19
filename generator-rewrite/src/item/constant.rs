use super::{Code, Context};
use crate::output::{CodeMap, Destination, ReexportAs};
use analysis::{
    item::constant::{Constant, ConstantType},
    rust::RustTokens,
    xml::cexpr::CExprItem,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Constant {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");

        let mut location = Destination::primary_location(self.required_by);
        let mut name = crate::constant_token(self.name.prefix_trimmed());

        for suffix in ["EXTENSION_NAME", "SPEC_VERSION"] {
            if self.name.original().ends_with(suffix) {
                let new_name = crate::constant_token(suffix);
                location.reexport = false;
                location.reexport_as.push(ReexportAs {
                    original: new_name.clone(),
                    alias: name,
                });

                name = new_name;
                break;
            }
        }

        let ty = match self.ty {
            ConstantType::Integer(primary_ty) => ctx.primary_type_tokens(primary_ty),
            ConstantType::String => quote! { &core::ffi::CStr },
        };

        let value = CExprItem::tokens(self.value.iter(), ctx);

        let code = quote! {
            pub const #name: #ty = #value;
        };

        CodeMap::new(location, code)
    }
}
