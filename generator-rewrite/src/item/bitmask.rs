use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{
        Named,
        bitmask::{BitMask, BitWidth, Item, Value},
    },
    lifetime::Lifetime,
    to_rust::RustTranslator,
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BitMask {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_to_rust(self.name(), false, &Lifetime::placeholder());
        let base_ty = match self.bitwidth {
            BitWidth::Bits32 => quote! { u32 },
            BitWidth::Bits64 => quote! { u64 },
        };

        let mut bits_code = TokenStream::default();
        let mut values = TokenStream::default();
        if let Some(bits_name) = self.bits_name {
            let bits_name_tokens = ctx.type_to_rust(bits_name, false, &Lifetime::placeholder());

            bits_code = quote! {
                #[repr(transparent)]
                #[derive(Clone, Copy)]
                pub struct #bits_name_tokens(pub(crate) #base_ty);
            };

            values = (self.items.iter())
                .map(|(&name, _item)| {
                    let name = ctx.enumerator_to_rust(name, bits_name, false);
                    quote! { pub const #name: Self = Self(#bits_name_tokens::#name.0); }
                })
                .collect::<TokenStream>();
        }

        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(#base_ty);

            impl #name {
                #values

                pub const fn empty() -> Self { 
                    Self(0)
                }

                pub const fn from_raw(x: #base_ty) -> Self {
                    Self(x)
                }

                pub const fn as_raw(self) -> #base_ty {
                    self.0
                }

                pub const fn is_empty(self) -> bool {
                    self.0 == Self::empty().0
                }

                pub const fn intersects(self, other: Self) -> bool {
                    !Self(self.0 & other.0).is_empty()
                }

                pub const fn contains(self, other: Self) -> bool {
                    self.0 & other.0 == other.0
                }
            }

            impl ::core::default::Default for #name {
                fn default() -> Self {
                    Self::empty()
                }
            }

            impl ::core::ops::BitOr for #name {
                type Output = Self;

                fn bitor(self, rhs: Self) -> Self {
                    Self(self.0 | rhs.0)
                }
            }

            impl ::core::ops::BitOrAssign for #name {
                fn bitor_assign(&mut self, rhs: Self) {
                    *self = *self | rhs;
                }
            }

            impl ::core::ops::BitAnd for #name {
                type Output = Self;

                fn bitand(self, rhs: Self) -> Self {
                    Self(self.0 & rhs.0)
                }
            }

            impl ::core::ops::BitAndAssign for #name {
                fn bitand_assign(&mut self, rhs: Self) {
                    *self = *self & rhs;
                }
            }

            impl ::core::ops::BitXor for #name {
                type Output = Self;

                fn bitxor(self, rhs: Self) -> Self {
                    Self(self.0 ^ rhs.0)
                }
            }

            impl ::core::ops::BitXorAssign for #name {
                fn bitxor_assign(&mut self, rhs: Self) {
                    *self = *self ^ rhs;
                }
            }

            impl ::core::ops::Not for #name {
                type Output = Self;

                fn not(self) -> Self {
                    Self(!self.0)
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
                let name = ctx.type_to_rust(
                    bits_name,
                    dest != Destination::new(self.required_by),
                    &Lifetime::placeholder(),
                );

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
