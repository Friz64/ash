use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    decl::Ty,
    item::{
        Named,
        structure::{Struct, StructMember, Union},
    },
    lifetime::Lifetime,
    name::TypeName,
    to_rust::RustTranslator,
};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let name = ctx.type_to_rust(self.name(), false, &lifetime);

        let lifetime_tok = ctx.type_has_lifetime(self.name()).then(|| quote! { #lifetime });
        let mut bitfield_i = 0;

        let mut contains_static_array = false;
        let members = (self.members.iter()).map(|member| match member {
            StructMember::Normal(decl) => {
                if let Ty::Array(_, _) = &decl.ty { contains_static_array = true }

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

        let lifetime_marker_val = ctx.type_has_lifetime(self.name()).then(|| {
            quote! { _marker: ::core::marker::PhantomData }
        });

        let tagged_structure = self.structure_type.as_ref().map(|ty| {
            let structure_ty = ctx.type_to_rust(TypeName::VK_STRUCTURE_TYPE, true, &lifetime);
            let ty = ctx.enumerator_to_rust(*ty, TypeName::VK_STRUCTURE_TYPE, true);
            quote! {
                unsafe impl<#lifetime> crate::TaggedStructure<#lifetime> for #name {
                    const STRUCTURE_TYPE: #structure_ty = #ty;
                }
            }
        });

        let repr = quote! {
            #[repr(C)]
        };

        let code = quote! {
            pub struct #name {
                #( #members, )*
                #lifetime_marker
            }

            #tagged_structure
        };

        bitfield_i = 0;
        let default = if contains_static_array || tagged_structure.is_some() {
            let defaults = self.members.iter().map(|member| match member {
                StructMember::Normal(decl) => {
                    let field_name = ctx.var_name_to_rust(decl.name);
                    if tagged_structure.is_some()
                        && decl.name.original() == "sType"
                        && let Ty::SpecType(ty) = &decl.ty
                        && ty == &TypeName::VK_STRUCTURE_TYPE
                    {
                        quote! { #field_name: <Self as crate::TaggedStructure>::STRUCTURE_TYPE }
                    } else if let Ty::Array(_, _) = &decl.ty {
                        quote! { #field_name: unsafe { core::mem::zeroed() } }
                    } else {
                        quote! { #field_name: Default::default() }
                    }
                }
                StructMember::BitField(_) => {
                    let name = format_ident!("bitfield{bitfield_i}");
                    bitfield_i += 1;
                    quote! { #name: Default::default() }
                }
            });

            Some(quote! {
                impl<#lifetime_tok> Default for #name {
                    fn default() -> Self {
                        Self {
                            #( #defaults, )*
                            #lifetime_marker_val
                        }
                    }
                }
            })
        } else {
            None
        };

        let derive_default = if default.is_none() { 
            Some(quote! {Default})
        } else { 
            None 
        };

        let derives = quote! {
            #[derive(Clone, Copy, #derive_default)]
        };

        let code = quote! {
            #repr 
            #derives
            #code
            #default
        };

        println!("{code}");

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

impl Code for Union {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let lifetime_tok = ctx.type_has_lifetime(self.name()).then(|| quote! { #lifetime });
        let name = ctx.type_to_rust(self.name(), false, &lifetime);
        let members = (self.members.iter()).map(|decl| decl.to_rust(ctx, &lifetime));

        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union #name {
                #( pub #members ),*
            }

            impl<#lifetime_tok> Default for #name {
                fn default() -> Self {
                    unsafe { core::mem::zeroed() }
                }
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
