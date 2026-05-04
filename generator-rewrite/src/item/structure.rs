use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    item::{
        Named,
        structure::{Struct, StructMember, Union},
    }, lifetime::Lifetime, name::TypeName, to_rust::RustTranslator
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let name = ctx.type_to_rust(self.name(), false, &lifetime);
        let mut bitfield_i = 0;
        let members = (self.members.iter()).map(|member| match member {
            StructMember::Normal(decl) => {
                let decl = decl.to_rust(ctx, &lifetime);
                quote! { pub #decl }
            }
            StructMember::BitField(ranges) => {
                let doc: String = ranges
                    .iter()
                    .map(|part| format!("- `{}` @ `{:?}`\n", part.decl.name.original(), part.range))
                    .collect();
                let doc = doc.trim_ascii_end();

                let name = format_ident!("bitfield{bitfield_i}");
                bitfield_i += 1;
                quote! {
                    #[doc = #doc]
                    pub #name: u32
                }
            }
        });

        let lifetime_marker = ctx.type_has_lifetime(self.name()).then(|| {
            quote! { pub _marker: ::core::marker::PhantomData<& #lifetime ()> }
        });

        let impl_tagged_structure = self.structure_type.as_ref().map(|ty| {
            let structure_ty = ctx.type_to_rust(TypeName::VK_STRUCTURE_TYPE, true, &lifetime);
            let ty = ctx.enumerator_to_rust(*ty, TypeName::VK_STRUCTURE_TYPE, true);
            quote! {
                unsafe impl<#lifetime> crate::TaggedStructure<#lifetime> for #name {
                    const STRUCTURE_TYPE: #structure_ty = #ty;
                }
            }
        });
        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub struct #name {
                #( #members, )*
                #lifetime_marker
            }

            #impl_tagged_structure
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for Union {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let name = ctx.type_to_rust(self.name(), false, &lifetime);
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx, &lifetime));

        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union #name {
                #( pub #members ),*
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
