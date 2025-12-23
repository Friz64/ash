use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::constant::{Constant, Value};
use proc_macro2::Literal;
use quote::{format_ident, quote};
use std::{ffi::CString, str::FromStr};

impl Code for Constant {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let (ty, value) = match self.value {
            Value::String(value) => {
                let literal = Literal::c_string(&CString::from_str(value).unwrap());
                (quote! { &'static core::ffi::CStr }, quote! { #literal })
            }
            Value::Todo => (quote! { usize }, quote! { 69 }),
        };

        let code = quote! {
            pub const #name: #ty = #value;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
