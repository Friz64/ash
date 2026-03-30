use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::bitmask::{BitMask, BitWidth, Item, Value},
    to_rust::{RustName, RustTranslator},
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
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
            quote! {
                #[repr(transparent)]
                #[derive(Clone, Copy)]
                pub struct #name(pub(crate) #base_ty);
            }
        });

        let values = self.bits_name.map(|bits_name| {
            let bits_name_tokens = ctx.type_to_rust(bits_name, false);
            self.items
                .iter()
                .map(|(&name, _item)| {
                    let name = ctx.enumerator_to_rust(name, bits_name, false);
                    quote! { const #name = #bits_name_tokens::#name.0; }
                })
                .collect::<TokenStream>()
        });

        let code = quote! {
            bitflags::bitflags! {
                #[repr(transparent)]
                #[derive(Clone, Copy)]
                pub struct #name: #base_ty {
                    #values
                }
            }

            #bits_code
        };

        let mut codemap = CodeMap::new(Destination::new(self.required_by), code);

        if let Some(bits_name) = self.bits_name {
            let mut impl_map = CodeMap::default();

            for (&name, Item { required_by, value }) in &self.items {
                let name = ctx.enumerator_to_rust(name, bits_name, false);
                let value = match &value {
                    Value::BitPos(bitpos) => {
                        let literal = Literal::u8_unsuffixed(*bitpos);
                        quote! { Self(1 << #literal) }
                    }
                    Value::Expr(cexpr_items) => {
                        let expr = CExprItem::to_rust(cexpr_items.iter(), ctx);
                        quote! { Self(#expr) }
                    }
                    Value::Alias(enumerator_name) => {
                        let en = ctx.enumerator_to_rust(*enumerator_name, bits_name, false);
                        quote! { Self::#en }
                    }
                };

                impl_map.extend(CodeMap::new(
                    Destination::new(*required_by),
                    quote! { pub const #name: Self = #value; },
                ));
            }

            for (&dest, impl_tokens) in impl_map.iter() {
                let name =
                    ctx.type_to_rust(bits_name, dest != Destination::new(self.required_by));
                let doc = dest.doc_link();
                codemap.extend(CodeMap::new(
                    dest,
                    quote! {
                        #[doc = #doc]
                        impl #name { #impl_tokens }
                    },
                ));
            }
        }

        codemap
    }
}
