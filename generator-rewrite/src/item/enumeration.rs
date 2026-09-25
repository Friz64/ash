use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::enumeration::{Enum, Value},
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
                .filter(|(_name, item)| !matches!(item.value, Value::Alias(..)))
                .map(|(&name, item)| {
                    let name = ctx.enumerator_tokens(name, self.name, false);
                    let name_string = name.to_string();

                    let is_provisional = item.required_by.primary_location().is_provisional(ctx);
                    let provisional_guard =
                        is_provisional.then_some(quote! { #[cfg(feature = "provisional")] });

                    quote! {
                        #provisional_guard
                        Self::#name => Some(#name_string),
                    }
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

        let display = (self.name == TypeName::VK_RESULT).then(|| {
            let display_items = (self.items.iter())
                .filter_map(|(name, item)| match (item.comment, &item.value) {
                    (_, Value::Alias(..)) => None, // filter out aliases
                    (Some(comment), _) => Some((name, crate::expand_doc_comment(comment))),
                    _ => None,
                })
                .map(|(&name, comment)| {
                    let name = ctx.enumerator_tokens(name, self.name, false);
                    quote! { Self::#name => Some(#comment), }
                });

            quote! {
                impl core::fmt::Display for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                        if let Some(x) = match *self {
                            #( #display_items )*
                            _ => None,
                        } {
                            f.write_str(x)
                        } else {
                            core::fmt::Debug::fmt(&self.0, f)
                        }
                    }
                }
            }
        });

        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            pub struct #name(pub(crate) i32);

            #debug
            #display
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

        for (&name, item) in &self.items {
            let name = ctx.enumerator_tokens(name, self.name, false);
            let value = match &item.value {
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

            let comment = item.comment.map(|comment| {
                let expanded = crate::expand_doc_comment(comment);
                quote! { #[doc = #expanded] }
            });

            impl_map.extend(CodeMap::new(
                Destination::primary_location(item.required_by),
                quote! {
                   #comment
                   pub const #name: Self = #value;
                },
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
                        #impl_tokens
                    }
                },
            ));
        }

        codemap
    }
}
