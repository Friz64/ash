use super::{Code, Context};
use crate::{
    Import,
    output::{CodeMap, GlobExport, OutputLocation},
};
use analysis::{
    item::enum_::{Enum, Value},
    name::TypeName,
    rust::RustTokens,
    xml::{Deprecated, cexpr::CExprItem},
};
use proc_macro2::Literal;
use quote::quote;
use tracing::{instrument, trace};

impl Code for Enum {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let name = octx.type_tokens(self.name, false, None);

        let filtered_items: Vec<_> = self
            .items
            .iter()
            // `Deprecated::Aliased` enums have degenerated names
            .filter(|(_name, item)| matches!(item.deprecated, None | Some(Deprecated::True)))
            .collect();

        let debug = {
            let debug_items = filtered_items
                .iter()
                .filter(|(_name, item)| !matches!(item.value, Value::Alias(..)))
                .map(|(name, item)| {
                    let name = octx.enumerator_tokens(**name, self.name, false);
                    let name_string = name.to_string();

                    let is_provisional = item.required_by.primary_location().is_provisional(ctx);
                    let provisional_guard =
                        is_provisional.then_some(quote! { #[cfg(feature = "provisional")] });

                    quote! {
                        #provisional_guard
                        Self::#name => Some(#name_string),
                    }
                });

            let unconditional_debug =
                self.name == TypeName::VK_RESULT || self.name == TypeName::VK_OBJECT_TYPE;
            let cfg_guard = (!unconditional_debug).then_some(quote! { #[cfg(feature = "debug")] });
            let fmt = if unconditional_debug {
                octx.insert_import(Import::CoreFmt);
                quote! { fmt }
            } else {
                octx.insert_import(Import::DebugCoreFmt);
                quote! { fmtd }
            };

            quote! {
                #cfg_guard
                impl #fmt::Debug for #name {
                    fn fmt(&self, f: &mut #fmt::Formatter) -> #fmt::Result {
                        if let Some(x) = match *self {
                            #( #debug_items )*
                            _ => None,
                        } {
                            f.write_str(x)
                        } else {
                            #fmt::Debug::fmt(&self.0, f)
                        }
                    }
                }
            }
        };

        let display = (self.name == TypeName::VK_RESULT).then(|| {
            let display_items = filtered_items
                .iter()
                .filter_map(|(name, item)| match (item.comment, &item.value) {
                    (_, Value::Alias(..)) => None, // filter out aliases
                    (Some(comment), _) => Some((name, crate::expand_doc_comment(comment))),
                    _ => None,
                })
                .map(|(name, comment)| {
                    let name = octx.enumerator_tokens(**name, self.name, false);
                    quote! { Self::#name => Some(#comment), }
                });

            octx.insert_import(Import::CoreFmt);
            quote! {
                impl fmt::Display for #name {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        if let Some(x) = match *self {
                            #( #display_items )*
                            _ => None,
                        } {
                            f.write_str(x)
                        } else {
                            fmt::Debug::fmt(&self.0, f)
                        }
                    }
                }
            }
        });

        octx.insert_tokens(quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            pub struct #name(pub(crate) i32);

            #debug
            #display

            impl #name {
                #[inline]
                pub const fn from_raw(x: i32) -> Self {
                    Self(x)
                }

                #[inline]
                pub const fn as_raw(self) -> i32 {
                    self.0
                }
            }
        });

        let mut impl_map = CodeMap::default();
        for (&name, item) in filtered_items {
            let output = OutputLocation::primary_location(item.required_by, GlobExport::No);
            let octx = impl_map.octx(output, ctx);

            let name = octx.enumerator_tokens(name, self.name, false);
            let value = match &item.value {
                Value::Variant(variant) => {
                    let literal = Literal::i32_unsuffixed(*variant);
                    quote! { Self(#literal) }
                }
                Value::Expr(cexpr_items) => {
                    let expr = CExprItem::tokens(cexpr_items.iter(), &octx);
                    quote! { Self(#expr) }
                }
                Value::Alias(enumerator_name) => {
                    let alias = octx.enumerator_tokens(*enumerator_name, self.name, false);
                    quote! { Self::#alias }
                }
            };

            let comment = item.comment.map(|comment| {
                let expanded = crate::expand_doc_comment(comment);
                quote! { #[doc = #expanded] }
            });

            octx.insert_tokens(quote! {
               #comment
               pub const #name: Self = #value;
            });
        }

        codemap.extend_mapped(impl_map, ctx, |octx, impl_tokens| {
            let doc = octx.output().provided_by_doc_comment();
            let name = octx.type_tokens(
                self.name,
                octx.output().require != self.required_by.primary_location(),
                None,
            );

            quote! {
                #[doc = #doc]
                impl #name {
                    #impl_tokens
                }
            }
        });
    }
}
