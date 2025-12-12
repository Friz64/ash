use super::Code;
use crate::{
    item,
    output::{CodeMap, Destination},
    util,
};
use analysis::item::structure::Structure;
use quote::{format_ident, quote};

impl Code for Structure {
    // TODO(friz64) fully implement.
    fn code(&self) -> CodeMap {
        let name = format_ident!("{}", self.name);
        let members = self.members.iter().map(|member| {
            let name = util::to_snake_case_escape_ident(member.name);
            let ty = item::ty_to_rust(&member.ty);
            quote! { pub #name: #ty, }
        });

        let code = quote! {
            #[repr(C)]
            pub struct #name {
                #( #members )*
            }
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
