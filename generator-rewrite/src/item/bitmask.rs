use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::bitmask::{BitMask, BitWidth, Item, Value},
    rust::{Lifetime, RustTokens},
    xml::cexpr::CExprItem,
};
use proc_macro2::{Literal, TokenStream};
use quote::quote;
use std::iter;
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

        let mut bits_definition = TokenStream::default();
        let mut bitmask_impl_map = CodeMap::default();
        let mut bits_impl_map = CodeMap::default();
        let mut debug_items = Vec::new();
        if let Some(bits_name) = self.bits_name {
            let bits_name_tokens = ctx.type_tokens(bits_name, false, &Lifetime::placeholder());
            bits_definition = quote! {
                #[repr(transparent)]
                #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
                pub struct #bits_name_tokens(pub(crate) #base_ty);

                impl #bits_name_tokens {
                    ///Converts this enum variant to the corresponding bitmask
                    pub const fn bitmask(&self) -> #name_tokens {
                        #name_tokens::from_raw(self.0)
                    }
                }

                #[cfg(feature = "debug")]
                impl core::fmt::Debug for #bits_name_tokens {
                    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                        core::fmt::Debug::fmt(&self.bitmask(), f)
                    }
                }
            };

            for (&name, Item { required_by, value }) in &self.items {
                let name = ctx.enumerator_tokens(name, bits_name, false);
                let dest = Destination::primary_location(*required_by);
                let value_tokens = match &value {
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

                bits_impl_map.extend(CodeMap::new(
                    dest.clone(),
                    quote! { pub const #name: Self = #value_tokens; },
                ));

                let bits_path = ctx.type_tokens(
                    bits_name,
                    dest != Destination::primary_location(self.required_by),
                    &Lifetime::placeholder(),
                );

                bitmask_impl_map.extend(CodeMap::new(
                    dest,
                    quote! { pub const #name: Self = Self(#bits_path::#name.0); },
                ));

                if !matches!(value, Value::Alias(..)) {
                    let is_provisional = required_by.primary_location().is_provisional(ctx);
                    let provisional_guard =
                        is_provisional.then_some(quote! { #[cfg(feature = "provisional")] });

                    let name_string = name.to_string();
                    debug_items.push(quote! {
                        #provisional_guard
                        (Self::#name.0, #name_string)
                    });
                }
            }
        }

        let debug_content = match debug_items.as_slice() {
            [] => quote! { core::fmt::Debug::fmt(&self.0, f) },
            debug_items => quote! { crate::debug_flags(f, &[ #( #debug_items, )* ], self.0) },
        };

        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct #name_tokens(pub(crate) #base_ty);

            #[cfg(feature = "debug")]
            impl core::fmt::Debug for #name_tokens {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                    #debug_content
                }
            }

            impl #name_tokens {
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

            #bits_definition
        };

        let mut codemap = CodeMap::new(Destination::primary_location(self.required_by), code);
        if let Some(bits_name) = self.bits_name {
            let bitmask_impls = iter::repeat(self.bitmask_name).zip(bitmask_impl_map.into_iter());
            let bits_impls = iter::repeat(bits_name).zip(bits_impl_map.into_iter());

            for (type_name, (mut dest, impl_tokens)) in bitmask_impls.chain(bits_impls) {
                let bits_path = ctx.type_tokens(
                    type_name,
                    dest != Destination::primary_location(self.required_by),
                    &Lifetime::placeholder(),
                );

                dest.reexport = false;
                let doc = dest.provided_by_doc_comment();
                codemap.extend(CodeMap::new(
                    dest,
                    quote! {
                        #[doc = #doc]
                        impl #bits_path { #impl_tokens }
                    },
                ));
            }
        }

        codemap
    }
}
