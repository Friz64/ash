use core::str::FromStr;

use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::structure::{Length, Member, RegularMember, Struct, Union},
    name::TypeName,
    rust::{Lifetime, RustTokens, RustTy},
};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use tracing::{instrument, trace};

impl Code for Struct {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let name = ctx.type_tokens(self.name, false, &lifetime);

        let lifetime_tok = ctx
            .type_has_lifetime(self.name)
            .then(|| quote! { #lifetime });
        let mut bitfield_i = 0;

        let mut contains_static_array = false;
        let members = (self.members.iter()).map(|member| match member {
            Member::Regular(RegularMember { decl, .. }) => {
                if let Ty::Array(_, _) = &decl.ty {
                    contains_static_array = true
                }

                let decl = decl.to_rust().tokens(ctx, &lifetime);
                quote! { pub #decl }
            }
            Member::Bitfield(ranges) => {
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

        let lifetime_marker = ctx.type_has_lifetime(self.name).then(|| {
            quote! { pub _marker: ::core::marker::PhantomData<& #lifetime ()> }
        });

        let lifetime_marker_val = ctx.type_has_lifetime(self.name).then(|| {
            quote! { _marker: ::core::marker::PhantomData }
        });

        let tagged_structure = self.structure_type.as_ref().map(|ty| {
            let structure_ty = ctx.type_tokens(TypeName::VK_STRUCTURE_TYPE, true, &lifetime);
            let ty = ctx.enumerator_tokens(*ty, TypeName::VK_STRUCTURE_TYPE, true);
            let anon = Lifetime::placeholder();
            let extends = self
                .extends
                .iter()
                .map(|ty| ctx.type_tokens(*ty, true, &anon));
            quote! {
                unsafe impl<#lifetime> crate::TaggedStructure<#lifetime> for #name {
                    const STRUCTURE_TYPE: #structure_ty = #ty;
                }

                #(unsafe impl<#lifetime_tok> crate::Extends<#extends> for #name {})*
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

        let mut bitfield_i = 0;
        let default = if contains_static_array || tagged_structure.is_some() {
            let defaults = self.members.iter().map(|member| match member {
                Member::Regular(RegularMember { decl, .. }) => {
                    let field_name = ctx.variable_token(decl.name);
                    if tagged_structure.is_some()
                        && decl.name.original() == "sType"
                        && let Ty::ApiType(ty) = &decl.ty
                        && ty == &TypeName::VK_STRUCTURE_TYPE
                    {
                        quote! { #field_name: <Self as crate::TaggedStructure>::STRUCTURE_TYPE }
                    } else if let Ty::Array(_, _) = &decl.ty {
                        quote! { #field_name: unsafe { core::mem::zeroed() } }
                    } else {
                        quote! { #field_name: Default::default() }
                    }
                }
                Member::Bitfield(_) => {
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

        let mut bitfield_i = 0;
        let builders = self
            .members
            .iter()
            .filter(|member| match member {
                Member::Regular(RegularMember { decl, .. }) => {
                    !matches!(decl.name.original(), "sType" | "pNext")
                }
                _ => true,
            })
            .flat_map(|member| match member {
                Member::Regular(member) => itertools::Either::Left(core::iter::once(
                    setter_and_getter(ctx, member, &lifetime),
                )),
                Member::Bitfield(bitfield_ranges) => {
                    let name = format_ident!("bitfield{bitfield_i}");
                    bitfield_i += 1;
                    itertools::Either::Right(bitfield_ranges.iter().map(move |member| {
                        let field_name = ctx.variable_token(member.decl.name);
                        let mask = {
                            let top = u32::MAX >> (u32::BITS - member.range.end as u32);
                            let bottom = u32::MAX << (member.range.start);
                            top & bottom
                        };
                        let mask_tok = Literal::from_str(&format!("0x{mask:08X}")).unwrap();
                        let mask_inv_tok = Literal::from_str(&format!("0x{:08X}", !mask)).unwrap();
                        let offset = member.range.start as u32;
                        let field_shift = if offset != 0 {
                            quote! { (#field_name << #offset)  }
                        } else {
                            quote! { #field_name }
                        };

                        let extract = if offset != 0 {
                            quote! { (self.#name & #mask_tok ) >> #offset }
                        } else {
                            quote! { self.#name & #mask_tok  }
                        };
                        let get_field_name = format_ident!("get_{field_name}");
                        quote! {
                            pub fn #field_name(mut self, #field_name: u32) -> Self {
                                let rest = self.#name & #mask_inv_tok;
                                self.#name = (#field_shift & #mask_tok) | rest;
                                self
                            }
                            pub fn #get_field_name(&self) -> u32 {
                                #extract
                            }
                        }
                    }))
                }
            });
        let code = quote! {
            #repr
            #derives
            #code
            #default

            impl<#lifetime_tok> #name {
                #(#builders)*
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}

fn setter_and_getter(
    ctx: &Context<'_>,
    member: &RegularMember,
    lifetime: &Lifetime,
) -> TokenStream {
    let field_name = ctx.variable_token(member.decl.name);

    let mut leading_p_count = 0;
    for c in member.decl.name.original().chars() {
        if c == 'p' {
            leading_p_count += 1;
        } else if c.is_ascii_uppercase() {
            break;
        } else {
            leading_p_count = 0;
            break;
        }
    }

    let mut method_name = (member.decl.name.original())
        .strip_prefix(&"p".repeat(leading_p_count))
        .unwrap()
        .to_owned();
    if member.length_at_depth(1) == Some(Length::Count(1)) {
        method_name += "_ptrs";
    }

    let method_name = crate::variable_token(&method_name);

    match member.decl.ty {
        Ty::ApiType(TypeName::VK_BOOL32) => {
            quote! {
                pub fn #field_name(mut self, #field_name: bool) -> Self {
                    self.#field_name = #field_name.into();
                    self
                }
            }
        }
        Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), mutability)
            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
        {
            let ty = RustTy::Ref(Box::new(RustTy::CStr), mutability).tokens(ctx, lifetime);
            let field_name_as_cstr = format_ident!("{field_name}_as_c_str");
            quote! {
                pub fn #field_name(mut self, #field_name: #ty) -> Self {
                    self.#field_name = #field_name.as_ptr();
                    self
                }

                pub unsafe fn #field_name_as_cstr(&self) -> Option<&core::ffi::CStr> {
                    if self.#field_name.is_null() {
                        None
                    } else {
                        Some(unsafe { core::ffi::CStr::from_ptr(self.#field_name) })
                    }
                }
            }
        }
        Ty::Array(Ty::CPrimary(CPrimaryType::Char), _)
            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
        {
            let field_name_as_cstr = format_ident!("{field_name}_as_c_str");
            quote! {
                pub fn #field_name(mut self, #field_name: &core::ffi::CStr) -> core::result::Result<Self, crate::CStrTooLargeForStaticArray> {
                    crate::write_c_str_slice_with_nul(&mut self.#field_name, #field_name).map(|_| self)
                }

                pub fn #field_name_as_cstr(&self) -> core::result::Result<&core::ffi::CStr, core::ffi::FromBytesUntilNulError> {
                    crate::wrap_c_str_slice_until_nul(&self.#field_name)
                }
            }
        }
        Ty::Array(element_ty, _)
            if let Some(Length::DefinedByMember(length_member)) = member.length_at_depth(0) =>
        {
            let length_name = ctx.variable_token(length_member);
            let field_name_as_slice = format_ident!("{field_name}_as_slice");
            let slice_ty = array_element_ty(member, element_ty);
            let slice = RustTy::Slice(Box::new(slice_ty), Mutability::Not, None)
                .tokens(ctx, &Lifetime::placeholder());

            quote! {
                pub fn #field_name(mut self, #field_name: #slice) -> Self {
                    self.#length_name = #field_name.len() as _;
                    self.#field_name[..#field_name.len()].copy_from_slice(#field_name);
                    self
                }

                pub fn #field_name_as_slice(&self) -> #slice {
                    &self.#field_name[..self.#length_name as _]
                }
            }
        }
        Ty::Ptr(element_ty, mutability)
            if let Some(Length::DefinedByMember(length_member)) = member.length_at_depth(0) =>
        {
            let mut ptr = match mutability {
                Mutability::Not => quote! { .as_ptr() },
                Mutability::Mut => quote! { .as_mut_ptr() },
            };

            let slice_ty = if let Ty::CPrimary(CPrimaryType::Void) = element_ty {
                ptr = quote! { #ptr.cast() };
                Ty::CPrimary(CPrimaryType::UInt8).to_rust()
            } else {
                if member.length_at_depth(1) == Some(Length::Count(1)) {
                    ptr = quote! { #ptr.cast() }
                }

                array_element_ty(member, element_ty)
            };

            let len_name = ctx.variable_token(length_member);
            let slice = RustTy::Slice(Box::new(slice_ty), mutability, None).tokens(ctx, lifetime);
            quote! {
                pub fn #method_name(mut self, #method_name: #slice) -> Self {
                    self.#len_name = #method_name.len() as _;
                    self.#field_name = #method_name #ptr;
                    self
                }
            }
        }
        Ty::Ptr(base, mutability)
            if member
                .length_at_depth(0)
                .is_none_or(|l| l == Length::Count(1)) =>
        {
            let ty = RustTy::Ref(Box::new(base.to_rust()), mutability).tokens(ctx, lifetime);
            quote! {
                pub fn #method_name(mut self, #method_name: #ty) -> Self {
                    self.#field_name = #method_name;
                    self
                }
            }
        }
        Ty::Ptr(base, mutability)
            if let Some(Length::Custom(custom)) = member.length_at_depth(0) =>
        {
            match *custom {
                _ => {
                    tracing::warn!(?custom, "unhandled custom length");
                    let ty = member.decl.ty.to_rust().tokens(ctx, lifetime);
                    quote! {
                        pub fn #field_name(mut self, #field_name: #ty) -> Self {
                            self.#field_name = #field_name;
                            self
                        }
                    }
                }
            }
        }
        _ => {
            let ty = member.decl.ty.to_rust().tokens(ctx, lifetime);
            quote! {
                pub fn #field_name(mut self, #field_name: #ty) -> Self {
                    self.#field_name = #field_name;
                    self
                }
            }
        }
    }
}

fn array_element_ty(member: &RegularMember, element_ty: &Ty) -> RustTy {
    match element_ty {
        Ty::Ptr(ty, mutability) if member.length_at_depth(1) == Some(Length::Count(1)) => {
            RustTy::Ref(Box::new(ty.to_rust()), *mutability)
        }
        ty @ (Ty::ApiType(_)
        | Ty::ApiFuncPointer(_)
        | Ty::CPrimary(_)
        | Ty::Array(_, _)
        | Ty::Ptr(_, _)
        | Ty::Platform(_)) => ty.to_rust(),
    }
}

impl Code for Union {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let lifetime = Lifetime(format_ident!("a"));
        let lifetime_tok = ctx
            .type_has_lifetime(self.name)
            .then(|| quote! { #lifetime });
        let name = ctx.type_tokens(self.name, false, &lifetime);
        let members = (self.members.iter()).map(|decl| decl.to_rust().tokens(ctx, &lifetime));

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
