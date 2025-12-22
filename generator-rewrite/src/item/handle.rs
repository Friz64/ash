use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::handle::Handle;
use quote::{format_ident, quote};

impl Code for Handle {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let code = quote! {
            #[repr(transparent)]
            pub struct #name(pub(crate) i32);
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
