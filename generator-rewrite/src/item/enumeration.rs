use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::enumeration::{Enum, Item, Value},
    name::TypeName,
    rust::{Lifetime, RustTokens},
    xml::cexpr::CExprItem,
};
use proc_macro2::Literal;
use quote::quote;
use tracing::{instrument, trace};

impl Code for Enum {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_tokens(self.name, false, &Lifetime::placeholder());

        let debug = {
            let debug_items = (self.items.iter())
                .filter_map(|(name, item)| {
                    (!matches!(item.value, Value::Alias(..))).then_some(name)
                })
                .map(|&name| {
                    let name = ctx.enumerator_tokens(name, self.name, false);
                    let name_string = name.to_string();
                    quote! { Self::#name => Some(#name_string), }
                });

            let cfg_guard = (self.name != TypeName::VK_RESULT
                && self.name != TypeName::VK_OBJECT_TYPE)
                .then_some(quote! { #[cfg(feature = "debug")] });

            quote! {
                #cfg_guard
                impl core::fmt::Debug for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                        if let Some(x) = match *self {
                            #( #debug_items )*
                            _ => None,
                        } {
                            f.write_str(x)
                        } else {
                            core::fmt::Debug::fmt(&self.0, f)
                        }
                    }
                }
            }
        };

        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            pub struct #name(pub(crate) i32);

            #debug
        };

        let mut codemap = CodeMap::new(Destination::primary_location(self.required_by), code);
        let mut impl_map = CodeMap::new(
            Destination::primary_location(self.required_by),
            quote! {
                #[inline]
                pub const fn from_raw(x: i32) -> Self {
                    Self(x)
                }

                #[inline]
                pub const fn as_raw(self) -> i32 {
                    self.0
                }
            },
        );

        for (&name, Item { required_by, value }) in &self.items {
            let name = ctx.enumerator_tokens(name, self.name, false);
            let value = match &value {
                Value::Variant(variant) => {
                    let literal = Literal::i32_unsuffixed(*variant);
                    quote! { Self(#literal) }
                }
                Value::Expr(cexpr_items) => {
                    let expr = CExprItem::tokens(cexpr_items.iter(), ctx);
                    quote! { Self(#expr) }
                }
                Value::Alias(enumerator_name) => {
                    let alias = ctx.enumerator_tokens(*enumerator_name, self.name, false);
                    quote! { Self::#alias }
                }
            };

            impl_map.extend(CodeMap::new(
                Destination::primary_location(*required_by),
                quote! { pub const #name: Self = #value; },
            ));
        }

        for (mut dest, impl_tokens) in impl_map.into_iter() {
            let name = ctx.type_tokens(
                self.name,
                dest != Destination::primary_location(self.required_by),
                &Lifetime::placeholder(),
            );

            dest.reexport = false;
            let doc = dest.doc_link();
            codemap.extend(CodeMap::new(
                dest,
                quote! {
                    #[doc = #doc]
                    impl #name {
                        // TODO: pull doc from xml
                        #impl_tokens
                    }
                },
            ));
        }

        /*
        codemap.extend(CodeMap::new(
            Destination::new(self.required_by),
            quote! {
                #[cfg(feature = "std")]
                impl std::error::Error for #name {}

                impl fmt::Display for Result {
                    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                        let name = match * self {  Self :: ERROR_UNKNOWN => Some ("An unknown error has occurred, due to an implementation or application bug") , _ => None , } ;
                        if let Some(x) = name {
                            fmt.write_str(x)
                        } else {
                            <Self as fmt::Debug>::fmt(self, fmt)
                        }
                    }
                }
            },
        ));
        */

        codemap
    }
}
