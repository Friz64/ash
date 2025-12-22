use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::basetype::BaseType;
use quote::{format_ident, quote};

impl Code for BaseType {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        // todo
        let code = quote! {
            #[repr(transparent)]
            #[allow(non_camel_case_types)]
            pub struct #name(pub(crate) i16);
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
