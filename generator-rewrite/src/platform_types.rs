use analysis::{platform_types::PLATFORM_TYPES, rust::RustTokens};
use proc_macro2::TokenStream;
use quote::quote;
use tracing::debug;

use crate::output::OutputContext;

pub fn generate_code() -> TokenStream {
    debug!("generating platform types code");

    let octx = OutputContext::dummy();
    let types = PLATFORM_TYPES.iter().map(|(name, ty)| {
        let ident = octx.platform_type_tokens(*name, false);
        let ty = ty.to_rust().tokens(&octx, None);
        quote! { pub type #ident = #ty; }
    });

    quote! {
        use crate::internal_prelude::*;
        #(#types)*
    }
}
