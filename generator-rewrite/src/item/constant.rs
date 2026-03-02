use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::constant::{Constant, ConstantType},
    to_rust::NameTranslate,
    xml::cexpr::CExprItem,
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Constant {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());

        let ty = match self.ty {
            ConstantType::Integer(primary_ty) => ctx.primary_type_to_rust(primary_ty),
            ConstantType::String => quote! { &core::ffi::CStr },
        };

        let value = CExprItem::to_rust(self.value.iter(), ctx);

        let code = quote! {
            pub const #name: #ty = #value;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
