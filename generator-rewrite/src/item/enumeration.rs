use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::enumeration::{Enum, Item, Value},
    to_rust::{RustName, RustTranslator},
    xml::cexpr::CExprItem,
};
use proc_macro2::Literal;
use quote::quote;
use tracing::{instrument, trace};

impl Code for Enum {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = self.rust_name(ctx);
        let code = quote! {
            #[repr(transparent)]
            #[derive(Clone, Copy)]
            pub struct #name(pub(crate) i32);
        };

        let mut codemap = CodeMap::new(Destination::new(self.required_by), code);
        let mut impl_map = CodeMap::default();

        for (&name, Item { required_by, value }) in &self.items {
            let name = ctx.enumerator_to_rust(name, self.name);
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
                    let ident = ctx.enumerator_to_rust(*enumerator_name, self.name);
                    quote! { Self::#ident }
                }
            };

            impl_map.extend(CodeMap::new(
                Destination::new(*required_by),
                quote! { pub const #name: Self = #value; },
            ));
        }

        for (&dest, impl_tokens) in impl_map.iter() {
            let name = ctx.type_to_rust(self.name, dest != Destination::new(self.required_by));
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
