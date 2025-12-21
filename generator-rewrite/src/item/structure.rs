use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::structure::{Struct, Union},
    to_rust::NameTranslate,
};
use quote::{format_ident, quote};

impl Code for Struct {
    fn code(&self, ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let members = self.members.iter().map(|decl| {
            let name = ctx.variable_to_rust(decl.name);
            let ty = decl.ty.to_rust(ctx);
            quote! { #name: #ty }
        });

        let code = quote! {
            #[repr(C)]
            pub struct #name {
                #( pub #members ),*
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}

impl Code for Union {
    fn code(&self, ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let members = self.members.iter().map(|decl| {
            let name = ctx.variable_to_rust(decl.name);
            let ty = decl.ty.to_rust(ctx);
            quote! { #name: #ty }
        });

        let code = quote! {
            #[repr(C)]
            pub struct #name {
                #( #members ),*
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
