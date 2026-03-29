use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::structure::{Struct, StructMember, Union},
    to_rust::RustName,
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let mut bitfield_i = 0;
        let members = (self.members.iter()).map(|member| match member {
            StructMember::Normal(decl) => {
                let decl = decl.to_rust(ctx);
                quote! { pub #decl }
            }
            StructMember::BitField(ranges) => {
                let doc: String = ranges
                    .iter()
                    .map(|part| format!("- `{}` @ `{:?}`\n", part.decl.name.original(), part.range))
                    .collect();
                let doc = doc.trim_ascii_end();

                let name = format_ident!("bitfield{bitfield_i}");
                bitfield_i += 1;
                quote! {
                    #[doc = #doc]
                    pub #name: u32
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

        CodeMap::new(Destination::library(self.required_by), code)
    }
}

impl Code for Union {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx));

        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union #name {
                #( pub #members ),*
            }
        };

        CodeMap::new(Destination::library(self.required_by), code)
    }
}
