use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::structure::{Struct, Union};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx));

        let code = quote! {
            #[repr(C)]
            pub struct #name {
                #( pub #members ),*
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}

impl Code for Union {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx));

        let code = quote! {
            #[repr(C)]
            pub struct #name {
                #( #members ),*
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
