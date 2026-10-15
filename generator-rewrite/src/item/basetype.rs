use super::{Code, Context};
use crate::output::{CodeMap, GlobExport, OutputLocation};
use analysis::{item::basetype::BaseType, rust::RustTokens};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BaseType {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let name = octx.type_tokens(self.name, false, None);
        let ty = self.ty.to_rust().tokens(&octx, None);
        let code = quote! {
            pub type #name = #ty;
        };

        octx.insert_tokens(code)
    }
}
