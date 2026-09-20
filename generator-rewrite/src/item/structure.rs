use core::str::FromStr;

use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::structure::{BitfieldMemberRange, Length, Member, RegularMember, Struct, Union},
    name::TypeName,
    rust::{Lifetime, RustTokens, RustTy},
};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::iter;
use syn::Ident;
use tracing::{instrument, trace};

fn method_name_token(ctx: &Context, member: &RegularMember) -> Ident {
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

    ctx.variable_token_from_original(&method_name)
}

fn as_c_str_method_token(method_name: &Ident) -> Ident {
    format_ident!("{method_name}_as_c_str")
}

fn bitfield_name_token(bitfield_i: usize) -> Ident {
    format_ident!("bitfield{bitfield_i}")
}

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
        let mut contains_bitfield = false;
        let members = (self.members.iter()).map(|member| match member {
            Member::Regular(RegularMember { decl, .. }) => {
                contains_static_array |= matches!(decl.ty, Ty::Array(..));

                let decl = decl.to_rust().tokens(ctx, &lifetime);
                quote! { pub #decl }
            }
            Member::Bitfield(ranges) => {
                contains_bitfield = true;

                let doc: String = ranges
                    .iter()
                    .map(|part| format!("- `{}` @ `{:?}`\n", part.decl.name.original(), part.range))
                    .collect();
                let doc = doc.trim_ascii_end();

                let name = bitfield_name_token(bitfield_i);
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
        let mut custom_default = false;
        let default = {
            let defaults = self.members.iter().map(|member| match member {
                Member::Regular(RegularMember { decl, .. }) => {
                    let field_name = ctx.variable_token(decl.name);
                    if tagged_structure.is_some()
                        && decl.name.original() == "sType"
                        && let Ty::ApiType(ty) = &decl.ty
                        && ty == &TypeName::VK_STRUCTURE_TYPE
                    {
                        custom_default = true;
                        quote! { #field_name: <Self as crate::TaggedStructure>::STRUCTURE_TYPE }
                    } else if let Ty::Array(_, _) = &decl.ty {
                        custom_default = true;
                        quote! { #field_name: unsafe { core::mem::zeroed() } }
                    } else {
                        quote! { #field_name: Default::default() }
                    }
                }
                Member::Bitfield(_) => {
                    custom_default = true;
                    let name = bitfield_name_token(bitfield_i);
                    bitfield_i += 1;
                    quote! { #name: Default::default() }
                }
            });

            quote! {
                impl<#lifetime_tok> Default for #name {
                    fn default() -> Self {
                        Self {
                            #( #defaults, )*
                            #lifetime_marker_val
                        }
                    }
                }
            }
        };

        let mut custom_debug = false;
        let debug = {
            let debug_fields = self.members.iter().map(|member| match member {
                Member::Regular(member @ RegularMember { decl, .. }) => {
                    let field_name = ctx.variable_token(decl.name);
                    let field_name_string = field_name.to_string();
                    let value = match decl.ty {
                        Ty::Array(Ty::CPrimary(CPrimaryType::Char), ..)
                            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
                        {
                            custom_debug = true;
                            let field_as_c_str =
                                as_c_str_method_token(&method_name_token(ctx, member));
                            quote! { &self.#field_as_c_str() }
                        }
                        _ => quote! { &self.#field_name },
                    };

                    quote! { .field(#field_name_string, #value) }
                }
                Member::Bitfield(bitfield_ranges) => {
                    custom_debug = true;
                    let debug_fields = bitfield_ranges.iter().map(|bitfield_member| {
                        let field_name = ctx.variable_token(bitfield_member.decl.name);
                        let field_name_string = field_name.to_string();
                        let get_field_name = format_ident!("get_{field_name}");
                        quote! { .field(#field_name_string, &self.#get_field_name()) }
                    });

                    quote! { #( #debug_fields )* }
                }
            });

            let name_string = name.to_string();
            quote! {
                impl<#lifetime_tok> core::fmt::Debug for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        f.debug_struct(#name_string)
                            #( #debug_fields )*
                            .finish()
                    }
                }
            }
        };

        let derive_default = (!custom_default).then_some(quote! {, Default});
        let default = custom_default.then_some(default);
        let derive_debug = (!custom_debug).then_some(quote! {, Debug});
        let debug = custom_debug.then_some(debug);
        let derives = quote! {
            #[derive(Clone, Copy #derive_default #derive_debug)]
        };

        let mut bitfield_i = 0;
        let builders = self.members.iter().flat_map(|member| match member {
            Member::Regular(member) => {
                itertools::Either::Left(iter::once(regular_builder(ctx, self, member, &lifetime)))
            }
            Member::Bitfield(bitfield_ranges) => {
                let bitfield_builder = bitfield_builder(ctx, bitfield_i, bitfield_ranges);
                bitfield_i += 1;
                itertools::Either::Right(bitfield_builder)
            }
        });

        let code = quote! {
            #repr
            #derives
            #code
            #default
            #debug

            impl<#lifetime_tok> #name {
                #(#builders)*
            }
        };

        CodeMap::new(Destination::primary_location(self.required_by), code)
    }
}

fn regular_builder(
    ctx: &Context,
    structure: &Struct,
    member: &RegularMember,
    lifetime: &Lifetime,
) -> TokenStream {
    if matches!(member.decl.name.original(), "sType" | "pNext") {
        return quote! {};
    }

    let field_name = ctx.variable_token(member.decl.name);

    let method_name = method_name_token(ctx, member);

    let mut skip_override = None;
    let mut ignore_custom_length = false;
    match (structure.name.original(), member.decl.name.original()) {
        // pViewports is allowed to be empty if the viewport state is empty
        ("VkPipelineViewportStateCreateInfo", "viewportCount") |
        // Must match viewportCount
        ("VkPipelineViewportStateCreateInfo", "scissorCount") |
        // descriptorCount is settable regardless of having pImmutableSamplers
        ("VkDescriptorSetLayoutBinding", "descriptorCount") |
        // No ImageView attachments when VK_FRAMEBUFFER_CREATE_IMAGELESS_BIT is set
        ("VkFramebufferCreateInfo", "attachmentCount") |
        // descriptorCount also describes descriptor length in pNext extension structures
        // https://github.com/ash-rs/ash/issues/806
        ("VkWriteDescriptorSet", "descriptorCount") => skip_override = Some(false),

        ("VkShaderModuleCreateInfo", "codeSize") => skip_override = Some(true),
        ("VkShaderModuleCreateInfo", "pCode") => return quote! {
            pub fn #method_name(mut self, #method_name: &#lifetime [u32]) -> Self {
                self.code_size = #method_name.len() * 4;
                self.p_code = #method_name.as_ptr();
                self
            }
        },

        ("VkAccelerationStructureVersionInfoKHR" | "VkMicromapVersionInfoEXT", "pVersionData") => return quote! {
            pub fn #method_name(mut self, #method_name: &#lifetime [u8; 2 * crate::vk::UUID_SIZE as usize]) -> Self {
                self.#field_name = #method_name.as_ptr();
                self
            }
        },

        ("VkPipelineMultisampleStateCreateInfo", "pSampleMask") |
        ("StdVideoH265HrdParameters", "pSubLayerHrdParametersNal" | "pSubLayerHrdParametersVcl")
            => ignore_custom_length = true,

        _ => (),
    }

    if skip_override.unwrap_or_else(|| {
        // does any member have its length defined by this member?
        structure.members.iter().any(|any_member| {
            if let Member::Regular(RegularMember { decl: _, lengths }) = any_member {
                lengths.iter().any(|length| {
                    if let Length::DefinedByMember(defined_by_member) = length {
                        defined_by_member == &member.decl.name
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        })
    }) {
        return TokenStream::new();
    }

    let array_element_ty = |element_ty: &Ty| {
        if let Ty::Ptr(ty, mutability) = element_ty
            && member.length_at_depth(1) == Some(Length::Count(1))
        {
            RustTy::Ref(Box::new(ty.to_rust()), *mutability)
        } else {
            element_ty.to_rust()
        }
    };

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
            let method_name_as_cstr = as_c_str_method_token(&method_name);
            quote! {
                pub fn #method_name(mut self, #method_name: #ty) -> Self {
                    self.#field_name = #method_name.as_ptr();
                    self
                }

                pub unsafe fn #method_name_as_cstr(&self) -> Option<&core::ffi::CStr> {
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
            let method_name_as_cstr = as_c_str_method_token(&method_name);
            quote! {
                pub fn #method_name(mut self, #method_name: &core::ffi::CStr) -> core::result::Result<Self, crate::CStrTooLargeForStaticArray> {
                    crate::write_c_str_slice_with_nul(&mut self.#field_name, #method_name).map(|_| self)
                }

                pub fn #method_name_as_cstr(&self) -> core::result::Result<&core::ffi::CStr, core::ffi::FromBytesUntilNulError> {
                    crate::wrap_c_str_slice_until_nul(&self.#field_name)
                }
            }
        }
        Ty::Array(element_ty, _)
            if let Some(Length::DefinedByMember(length_member)) = member.length_at_depth(0) =>
        {
            let length_name = ctx.variable_token(length_member);
            let method_name_as_slice = format_ident!("{method_name}_as_slice");
            let slice_ty = array_element_ty(element_ty);
            let slice = RustTy::Slice(Box::new(slice_ty), Mutability::Not, None)
                .tokens(ctx, &Lifetime::placeholder());

            quote! {
                pub fn #field_name(mut self, #method_name: #slice) -> Self {
                    self.#length_name = #method_name.len() as _;
                    self.#field_name[..#method_name.len()].copy_from_slice(#method_name);
                    self
                }

                pub fn #method_name_as_slice(&self) -> #slice {
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

                array_element_ty(element_ty)
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
        _ if let Some(Length::Custom(custom)) = member.length_at_depth(0)
            && !ignore_custom_length =>
        {
            panic!("unhandled custom length {custom:?}");
        }
        _ => {
            let ty = member.decl.ty.to_rust().tokens(ctx, lifetime);
            quote! {
                pub fn #method_name(mut self, #method_name: #ty) -> Self {
                    self.#field_name = #method_name;
                    self
                }
            }
        }
    }
}

fn bitfield_builder(
    ctx: &Context,
    bitfield_i: usize,
    bitfield_ranges: &[BitfieldMemberRange],
) -> impl Iterator<Item = TokenStream> {
    let name = bitfield_name_token(bitfield_i);
    bitfield_ranges.iter().map(move |member| {
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
    })
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

        let name_str = self.name.original();
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

            impl<#lifetime_tok> core::fmt::Debug for #name {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, #name_str)
                }
            }
        };

        CodeMap::new(Destination::primary_location(self.required_by), code)
    }
}
