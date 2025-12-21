use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::bitmask::{BitMask, BitMaskBits};
use quote::{format_ident, quote};

impl Code for BitMask {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let code = quote! {
            #[repr(C)]
            pub struct #name {
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}

impl Code for BitMaskBits {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let code = quote! {
            #[repr(C)]
            pub struct #name {
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
