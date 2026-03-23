use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::bitmask::{BitMask, BitWidth},
    to_rust::{RustName, RustTranslator},
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for BitMask {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let base_ty = match self.bitwidth {
            BitWidth::Bits32 => quote! { u32 },
            BitWidth::Bits64 => quote! { u64 },
        };

        let bits_code = self.bits_name.map(|bits_name| {
            let name = ctx.type_to_rust(bits_name, false);
            let values = self.values.iter().map(|value| {
                let name = format_ident!("{}", value.stripped_name(bits_name));
                quote! { pub const #name: Self = Self(1); }
            });

            quote! {
                #[repr(transparent)]
                #[derive(Clone, Copy)]
                pub struct #name(pub(crate) #base_ty);

                impl #name {
                    #( #values )*
                }
            }
        });

        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) #base_ty);

            #bits_code
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
