use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{
        Named,
        enumeration::{Enum, Item, Value},
    },
    lifetime::Lifetime,
    to_rust::RustTranslator,
    xml::cexpr::CExprItem,
};
use proc_macro2::Literal;
use quote::quote;
use tracing::{instrument, trace};

impl Code for Enum {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_to_rust(self.name(), false, &Lifetime::placeholder());
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            #[derive(Debug)] // TODO: proper impl
            pub struct #name(pub(crate) i32);
        };

        let mut codemap = CodeMap::new(Destination::new(self.required_by), code);
        let mut impl_map = CodeMap::default();

        for (&name, Item { required_by, value }) in &self.items {
            let name = ctx.enumerator_to_rust(name, self.name, false);
            let value = match &value {
                Value::Variant(variant) => {
                    let literal = Literal::i32_unsuffixed(*variant);
                    quote! { Self(#literal) }
                }
                Value::Expr(cexpr_items) => {
                    let expr = CExprItem::to_rust(cexpr_items.iter(), ctx);
                    quote! { Self(#expr) }
                }
                Value::Alias(enumerator_name) => {
                    let alias = ctx.enumerator_to_rust(*enumerator_name, self.name, false);
                    quote! { Self::#alias }
                }
            };

            impl_map.extend(CodeMap::new(
                Destination::new(*required_by),
                quote! { pub const #name: Self = #value; },
            ));
        }

        for (&dest, impl_tokens) in impl_map.iter() {
            let name = ctx.type_to_rust(
                self.name,
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

        codemap
    }
}
