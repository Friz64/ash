use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::bitmask::{BitMask, BitWidth, Item, Value},
    rust::{Lifetime, RustTokens},
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use tracing::{instrument, trace};

impl Code for BitMask {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name_tokens = ctx.type_tokens(self.bitmask_name, false, &Lifetime::placeholder());
        let base_ty = match self.bitwidth {
            BitWidth::Bits32 => quote! { u32 },
            BitWidth::Bits64 => quote! { u64 },
        };

        let mut bits_code = TokenStream::default();
        let mut values = TokenStream::default();
        let mut debug_content = None;
        if let Some(bits_name) = self.bits_name {
            let bits_name_tokens = ctx.type_tokens(bits_name, false, &Lifetime::placeholder());

            let bits_debug = {
                let content = if self.items.is_empty() {
                    quote! { core::fmt::Debug::fmt(&self.0, f) }
                } else {
                    let debug_items = (self.items.iter())
                        .filter_map(|(name, item)| {
                            (!matches!(item.value, Value::Alias(..))).then_some(name)
                        })
                        .map(|&name| {
                            let name = ctx.enumerator_tokens(name, bits_name, false);
                            let name_string = name.to_string();
                            quote! { Self::#name => Some(#name_string), }
                        });

                    quote! {
                        if let Some(x) = match *self {
                            #( #debug_items )*
                            _ => None,
                        } {
                            f.write_str(x)
                        } else {
                            core::fmt::Debug::fmt(&self.0, f)
                        }
                    }
                };

                quote! {
                    #[cfg(feature = "debug")]
                    impl core::fmt::Debug for #bits_name_tokens {
                        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                            #content
                        }
                    }
                }
            };

            bits_code = quote! {
                #[repr(transparent)]
                #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #bits_name_tokens(pub(crate) #base_ty);

                #bits_debug
            };

            values = (self.items.iter())
                .map(|(&name, _item)| {
                    let name = ctx.enumerator_tokens(name, bits_name, false);
                    quote! { pub const #name: Self = Self(#bits_name_tokens::#name.0); }
                })
                .collect::<TokenStream>();

            debug_content = (!self.items.is_empty()).then(|| {
                let debug_items = (self.items.iter())
                    .filter_map(|(name, item)| {
                        (!matches!(item.value, Value::Alias(..))).then_some(name)
                    })
                    .map(|&name| {
                        let name = ctx.enumerator_tokens(name, bits_name, false);
                        let name_string = name.to_string();
                        quote! { (Self::#name.0, #name_string) }
                    });

                quote! {
                    crate::debug_flags(f, &[ #( #debug_items, )* ], self.0)
                }
            });
        }

        let debug_content = debug_content.unwrap_or(quote! { core::fmt::Debug::fmt(&self.0, f) });
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #name_tokens(#base_ty);

            #[cfg(feature = "debug")]
            impl core::fmt::Debug for #name_tokens {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                    #debug_content
                }
            }

            impl #name_tokens {
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

            impl core::ops::BitOr for #name_tokens {
                type Output = Self;

                fn bitor(self, rhs: Self) -> Self {
                    Self(self.0 | rhs.0)
                }
            }

            impl core::ops::BitOrAssign for #name_tokens {
                fn bitor_assign(&mut self, rhs: Self) {
                    *self = *self | rhs;
                }
            }

            impl core::ops::BitAnd for #name_tokens {
                type Output = Self;

                fn bitand(self, rhs: Self) -> Self {
                    Self(self.0 & rhs.0)
                }
            }

            impl core::ops::BitAndAssign for #name_tokens {
                fn bitand_assign(&mut self, rhs: Self) {
                    *self = *self & rhs;
                }
            }

            impl core::ops::BitXor for #name_tokens {
                type Output = Self;

                fn bitxor(self, rhs: Self) -> Self {
                    Self(self.0 ^ rhs.0)
                }
            }

            impl core::ops::BitXorAssign for #name_tokens {
                fn bitxor_assign(&mut self, rhs: Self) {
                    *self = *self ^ rhs;
                }
            }

            impl core::ops::Not for #name_tokens {
                type Output = Self;

                fn not(self) -> Self {
                    Self(!self.0)
                }
            }

            #bits_code
        };

        let mut codemap = CodeMap::new(Destination::primary_location(self.required_by), code);

        if let Some(bits_name) = self.bits_name {
            let mut impl_map = CodeMap::default();

            for (&name, Item { required_by, value }) in &self.items {
                let name = ctx.enumerator_tokens(name, bits_name, false);
                let value = match &value {
                    Value::BitPos(bitpos) => {
                        let literal = Literal::u8_unsuffixed(*bitpos);
                        quote! { Self(1 << #literal) }
                    }
                    Value::Expr(cexpr_items) => {
                        let expr = CExprItem::tokens(cexpr_items.iter(), ctx);
                        quote! { Self(#expr) }
                    }
                    Value::Alias(enumerator_name) => {
                        let en = ctx.enumerator_tokens(*enumerator_name, bits_name, false);
                        quote! { Self::#en }
                    }
                };

                impl_map.extend(CodeMap::new(
                    Destination::primary_location(*required_by),
                    quote! { pub const #name: Self = #value; },
                ));
            }

            for (mut dest, impl_tokens) in impl_map.into_iter() {
                let name = ctx.type_tokens(
                    bits_name,
                    dest != Destination::primary_location(self.required_by),
                    &Lifetime::placeholder(),
                );

                dest.reexport = false;
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
