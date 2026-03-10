use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::structure::{Struct, StructMember, Union},
    to_rust::RustTranslator,
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let mut bitfield_i = 0;
        let members = (self.members.iter()).map(|member| match member {
            StructMember::Normal(decl) => {
                let decl = decl.to_rust(ctx);
                quote! { pub #decl }
            }
            StructMember::BitField { ty, ranges } => {
                let doc: String = ranges
                    .iter()
                    .map(|part| {
                        format!(
                            "- `{}` @ `{}..{}`\n",
                            part.name.original(),
                            part.range.start,
                            part.range.end
                        )
                    })
                    .collect();

                let name = format_ident!("bitfield{bitfield_i}");
                bitfield_i += 1;
                let ty = ty.to_rust(ctx);
                quote! {
                    #[doc = #doc]
                    pub #name: #ty
                }
            }
        });

        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub struct #name {
                #( #members ),*
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for Union {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.rust_name_of_type(self);
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx));

        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union #name {
                #( pub #members ),*
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
