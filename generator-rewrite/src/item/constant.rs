use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::constant::{Constant, Value},
    to_rust::NameTranslate,
};
use proc_macro2::Literal;
use quote::{format_ident, quote};
use std::{ffi::CString, str::FromStr};
use tracing::{instrument, trace};

impl Code for Constant {
    #[instrument]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let (ty, value) = match &self.value {
            Value::LiteralString(value) => {
                let literal = Literal::c_string(&CString::from_str(value).unwrap());
                (quote! { &core::ffi::CStr }, quote! { #literal })
            }
            Value::Expression(ty, expression) => {
                let ty = match ty {
                    Some(primary_ty) => ctx.primary_type_to_rust(*primary_ty),
                    None => quote! { usize },
                };

                (ty, expression.to_rust(ctx))
            }
        };

        let code = quote! {
            pub const #name: #ty = #value;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
