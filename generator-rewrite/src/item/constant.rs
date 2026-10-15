use super::{Code, Context};
use crate::{
    Import,
    output::{CodeMap, GlobExport, OutputLocation},
};
use analysis::{
    item::constant::{Constant, ConstantType},
    rust::RustTokens,
    xml::cexpr::CExprItem,
};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Constant {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");

        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let mut name = octx.constant_token(self.name);

        for special in ["EXTENSION_NAME", "SPEC_VERSION"] {
            if self.name.original().ends_with(special) {
                let new_name = octx.constant_token_from_prefix_stripped(special);
                octx.insert_reexport(new_name.clone(), name);
                name = new_name;
                break;
            }
        }

        let ty = match self.ty {
            ConstantType::Integer(primary_ty) => octx.primary_type_tokens(primary_ty),
            ConstantType::String => {
                octx.insert_import(Import::CoreFfiCStr);
                quote! { &CStr }
            }
        };

        let value = CExprItem::tokens(self.value.iter(), &octx);

        let code = quote! {
            pub const #name: #ty = #value;
        };

        octx.insert_tokens(code)
    }
}
