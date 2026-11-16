use crate::Context;
use analysis::{platform_types::PLATFORM_TYPES, rust::RustTokens};
use proc_macro2::TokenStream;
use quote::quote;
use tracing::debug;

pub fn generate_code(ctx: &Context) -> TokenStream {
    debug!("generating platform types code");

    let types = PLATFORM_TYPES.iter().map(|(name, ty)| {
        let ident = ctx.platform_type_tokens(*name, false);
        let ty = ty.to_rust().tokens(ctx, None);
        quote! { pub type #ident = #ty; }
    });

    quote! {
        use crate::internal_prelude::*;
        #(#types)*
    }
}
