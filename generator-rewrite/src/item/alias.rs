use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::item::alias::Alias;
use quote::{format_ident, quote};

impl Code for Alias {
    fn code(&self, _ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let alias = format_ident!("{}", self.alias.prefix_trimmed());
        // todo: is putting crate::vk:: everywhere really the solution?????????
        let code = quote! {
            pub type #name = crate::vk::#alias;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
