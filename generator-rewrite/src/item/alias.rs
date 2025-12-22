use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::alias::Alias, to_rust::NameTranslate};
use quote::{format_ident, quote};

impl Code for Alias {
    fn code(&self, ctx: &Context) -> CodeMap {
        let name = format_ident!("{}", self.name.prefix_trimmed());
        let alias = ctx.spec_type_to_rust(self.alias);
        let code = quote! {
            pub type #name = #alias;
        };

        CodeMap::new(Destination(self.required_by), code)
    }
}
