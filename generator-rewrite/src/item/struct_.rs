use super::{Code, Context};
use crate::{
    Function, OutputContext, Safety,
    output::{CodeMap, GlobExport, OutputLocation},
    rules::BuilderRule,
};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::struct_::{BitfieldMemberRange, Length, Member, RegularMember, Struct, Union},
    name::TypeName,
    rust::{Lifetime, RustTokens, RustTy},
};
use heck::ToSnekCase;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::str::FromStr;
use syn::Ident;
use tracing::{instrument, trace};

fn method_name_token(member: &RegularMember) -> Ident {
    let mut method_name = crate::strip_leading_p(member.decl.name.original()).to_owned();
    if member.length_at_depth(1) == Some(Length::Count(1)) {
        method_name += "_ptrs";
    }

    crate::escape_ident(&method_name.to_snek_case())
}

fn as_c_str_method_token(method_name: &Ident) -> Ident {
    format_ident!("{method_name}_as_c_str")
}

fn bitfield_name_token(bitfield_i: usize) -> Ident {
    format_ident!("bitfield{bitfield_i}")
}

impl Code for Struct {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_of(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let lifetime = Lifetime(format_ident!("a"));
        let name = octx.type_tokens(self.name, false, Some(&lifetime));
        let anon_name = octx.type_tokens(self.name, false, None);

        let lifetime_brackets = ctx
            .type_has_lifetime(self.name)
            .then(|| quote! { <#lifetime> });
        let mut bitfield_i = 0;

        let mut contains_static_array = false;
        let mut contains_bitfield = false;
        let members = (self.members.iter()).map(|member| match member {
            Member::Regular(RegularMember { decl, .. }) => {
                contains_static_array |= matches!(decl.ty, Ty::Array(..));

                let decl = decl.to_rust().tokens(&octx, Some(&lifetime));
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

        let mut lifetime_marker = TokenStream::new();
        let mut lifetime_marker_val = TokenStream::new();
        if ctx.type_has_lifetime(self.name) {
            lifetime_marker = quote! { pub _marker: PhantomData<& #lifetime ()> };
            lifetime_marker_val = quote! { _marker: PhantomData };
        }

        let mut structure_type_enum = None;
        let tagged_structure = self.structure_type.as_ref().map(|ty| {
            let structure_ty = octx.type_tokens(TypeName::VK_STRUCTURE_TYPE, true, Some(&lifetime));
            let enum_ = octx.enumerator_tokens(*ty, TypeName::VK_STRUCTURE_TYPE, true);
            structure_type_enum = Some(enum_.clone());
            let extends = self.extends.iter().map(|ty| {
                let is_provisional = ctx
                    .type_required_by(*ty)
                    .primary_location()
                    .is_provisional(ctx);
                let provisional_guard =
                    is_provisional.then_some(quote! { #[cfg(feature = "provisional")] });

                let extends = octx.type_tokens(*ty, true, None);
                quote! {
                    #provisional_guard
                    unsafe impl #lifetime_brackets Extends<#extends> for #name {}
                }
            });

            quote! {
                unsafe impl<#lifetime> TaggedStructure<#lifetime> for #name {
                    const STRUCTURE_TYPE: #structure_ty = #enum_;
                }

                #(#extends)*
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

            unsafe impl Send for #anon_name {}
            unsafe impl Sync for #anon_name {}

            #tagged_structure
        };

        let mut bitfield_i = 0;
        let mut custom_default = false;
        let default = {
            let defaults = self.members.iter().map(|member| match member {
                Member::Regular(RegularMember { decl, .. }) => {
                    let field_name = octx.variable_token(decl.name);
                    if let Some(enum_) = structure_type_enum.take()
                        && decl.name.original() == "sType"
                    {
                        custom_default = true;
                        quote! { #field_name: #enum_ }
                    } else if let Ty::Array(_, _) = &decl.ty {
                        custom_default = true;
                        quote! { #field_name: unsafe { zeroed() } }
                    } else {
                        quote! { #field_name: Default::default() }
                    }
                }
                Member::Bitfield(_) => {
                    let name = bitfield_name_token(bitfield_i);
                    bitfield_i += 1;
                    quote! { #name: Default::default() }
                }
            });

            quote! {
                impl #lifetime_brackets Default for #name {
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
                    let field_name = octx.variable_token(decl.name);
                    let field_name_string = field_name.to_string();
                    let value = match decl.ty {
                        Ty::Array(Ty::CPrimary(CPrimaryType::Char), ..)
                            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
                        {
                            custom_debug = true;
                            let field_as_c_str = as_c_str_method_token(&method_name_token(member));
                            quote! { &self.#field_as_c_str() }
                        }
                        _ => quote! { &self.#field_name },
                    };

                    quote! { .field(#field_name_string, #value) }
                }
                Member::Bitfield(bitfield_ranges) => {
                    custom_debug = true;
                    let debug_fields = bitfield_ranges.iter().map(|bitfield_member| {
                        let field_name = octx.variable_token(bitfield_member.decl.name);
                        let field_name_string = field_name.to_string();
                        let get_field_name = format_ident!("get_{field_name}");
                        quote! { .field(#field_name_string, &self.#get_field_name()) }
                    });

                    quote! { #( #debug_fields )* }
                }
            });

            let name_string = name.to_string();
            quote! {
                #[cfg(feature = "debug")]
                impl #lifetime_brackets fmtd::Debug for #name {
                    fn fmt(&self, f: &mut fmtd::Formatter) -> fmtd::Result {
                        f.debug_struct(#name_string)
                            #( #debug_fields )*
                            .finish()
                    }
                }
            }
        };

        let derive_default = (!custom_default).then_some(quote! {, Default});
        let default = custom_default.then_some(default);
        let derive_debug =
            (!custom_debug).then_some(quote! { #[cfg_attr(feature = "debug", derive(Debug))] });
        let debug = custom_debug.then_some(debug);

        let derives = quote! {
            #derive_debug
            #[derive(Clone, Copy #derive_default)]
        };

        let mut bitfield_i = 0;
        let builders = self.members.iter().flat_map(|member| match member {
            Member::Regular(member) => {
                let methods = match ctx.builder_rules.get(self.name, member.decl.name) {
                    _ if matches!(member.decl.name.original(), "sType" | "pNext") => vec![],
                    Some(BuilderRule::Skip) => vec![],
                    Some(BuilderRule::Replace(methods)) => methods.clone(),
                    rule @ (Some(BuilderRule::Keep) | None) => {
                        let keep = matches!(rule, Some(BuilderRule::Keep));
                        regular_builder(&octx, member, &lifetime, keep)
                    }
                };

                itertools::Either::Left(methods.into_iter().map(
                    |Function {
                         attributes,
                         safety,
                         name_override,
                         inputs,
                         output,
                         content,
                     }| {
                        let safety = match safety {
                            Safety::Default | Safety::Safe => quote! {},
                            Safety::Unsafe => quote! { unsafe },
                        };

                        let name = match name_override {
                            Some(name) => name.clone(),
                            None => method_name_token(member),
                        };

                        quote! {
                           #attributes
                           pub #safety fn #name(#inputs) #output { #content }
                        }
                    },
                ))
            }
            Member::Bitfield(bitfield_ranges) => {
                let methods = bitfield_builder(&octx, bitfield_i, bitfield_ranges);
                bitfield_i += 1;
                itertools::Either::Right(methods)
            }
        });

        let code = quote! {
            #repr
            #derives
            #code
            #default
            #debug

            impl #lifetime_brackets #name {
                #(#builders)*
            }
        };

        octx.emit_tokens(code)
    }
}

fn regular_builder(
    octx: &OutputContext,
    member: &RegularMember,
    lifetime: &Lifetime,
    keep: bool,
) -> Vec<Function> {
    let field_name = octx.variable_token(member.decl.name);
    let method_name = method_name_token(member);

    let is_length_method = !member.length_of.is_empty();
    if !keep && is_length_method {
        return vec![];
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
            vec![Function {
                inputs: quote! { mut self, #field_name: bool },
                output: quote! { -> Self },
                content: quote! {  self.#field_name = #field_name.into(); self },
                ..Default::default()
            }]
        }
        Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), mutability)
            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
        {
            let ty = RustTy::Ref(
                Box::new(RustTy::Custom {
                    custom_type: quote! { CStr },
                    generic_args: vec![],
                }),
                mutability,
            )
            .tokens(octx, Some(lifetime));

            vec![
                Function {
                    inputs: quote! { mut self, #method_name: #ty },
                    output: quote! { -> Self },
                    content: quote! { self.#field_name = #method_name.as_ptr(); self },
                    ..Default::default()
                },
                Function {
                    safety: Safety::Unsafe,
                    name_override: Some(as_c_str_method_token(&method_name)),
                    inputs: quote! { &self },
                    output: quote! { -> Option<&CStr> },
                    content: quote! {
                        if self.#field_name.is_null() {
                            None
                        } else {
                            Some(CStr::from_ptr(self.#field_name))
                        }
                    },
                    ..Default::default()
                },
            ]
        }
        Ty::Array(Ty::CPrimary(CPrimaryType::Char), _)
            if member.length_at_depth(0) == Some(Length::NullTerminated) =>
        {
            vec![
                Function {
                    inputs: quote! { mut self, #method_name: &CStr },
                    output: quote! { -> CoreResult<Self, crate::CStrTooLargeForStaticArray> },
                    content: quote! {
                        crate::write_c_str_slice_with_nul(&mut self.#field_name, #method_name)
                            .map(|_| self)
                    },
                    ..Default::default()
                },
                Function {
                    name_override: Some(as_c_str_method_token(&method_name)),
                    inputs: quote! { &self },
                    output: quote! { -> CoreResult<&CStr, ffi::FromBytesUntilNulError> },
                    content: quote! { crate::wrap_c_str_slice_until_nul(&self.#field_name) },
                    ..Default::default()
                },
            ]
        }
        Ty::Array(element_ty, _)
            if let Some(Length::DefinedByMember(length_member)) = member.length_at_depth(0) =>
        {
            let length_name = octx.variable_token(length_member);
            let slice_ty = array_element_ty(element_ty);
            let slice =
                RustTy::ArrayRef(Box::new(slice_ty), Mutability::Not, None).tokens(octx, None);
            vec![
                Function {
                    inputs: quote! { mut self, #method_name: #slice },
                    output: quote! { -> Self },
                    content: quote! {
                        self.#length_name = #method_name.len() as _;
                        self.#field_name[..#method_name.len()].copy_from_slice(#method_name);
                        self
                    },
                    ..Default::default()
                },
                Function {
                    name_override: Some(format_ident!("{method_name}_as_slice")),
                    inputs: quote! { &self },
                    output: quote! { -> #slice },
                    content: quote! { &self.#field_name[..self.#length_name as _] },
                    ..Default::default()
                },
            ]
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

            let len_name = octx.variable_token(length_member);
            let slice =
                RustTy::ArrayRef(Box::new(slice_ty), mutability, None).tokens(octx, Some(lifetime));
            vec![Function {
                inputs: quote! { mut self, #method_name: #slice },
                output: quote! { -> Self },
                content: quote! {
                    self.#len_name = #method_name.len() as _;
                    self.#field_name = #method_name #ptr;
                    self
                },
                ..Default::default()
            }]
        }
        Ty::Ptr(base, mutability)
            if member
                .length_at_depth(0)
                .is_none_or(|l| l == Length::Count(1)) =>
        {
            let ty = RustTy::Ref(Box::new(base.to_rust()), mutability).tokens(octx, Some(lifetime));
            vec![Function {
                inputs: quote! { mut self, #method_name: #ty },
                output: quote! { -> Self },
                content: quote! { self.#field_name = #method_name; self },
                ..Default::default()
            }]
        }
        _ => {
            let ty = member.decl.ty.to_rust().tokens(octx, Some(lifetime));
            vec![Function {
                inputs: quote! { mut self, #method_name: #ty },
                output: quote! { -> Self },
                content: quote! { self.#field_name = #method_name; self },
                ..Default::default()
            }]
        }
    }
}

fn bitfield_builder(
    octx: &OutputContext,
    bitfield_i: usize,
    bitfield_ranges: &[BitfieldMemberRange],
) -> impl Iterator<Item = TokenStream> {
    let name = bitfield_name_token(bitfield_i);
    bitfield_ranges.iter().map(move |member| {
        let field_name = octx.variable_token(member.decl.name);
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
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");

        let output = OutputLocation::primary_of(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);

        let lifetime = Lifetime(format_ident!("a"));
        let lifetime_brackets = ctx
            .type_has_lifetime(self.name)
            .then(|| quote! { <#lifetime> });
        let name = octx.type_tokens(self.name, false, Some(&lifetime));
        let members =
            (self.members.iter()).map(|decl| decl.to_rust().tokens(&octx, Some(&lifetime)));

        let name_string = name.to_string();
        let code = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            pub union #name {
                #( pub #members ),*
            }

            impl #lifetime_brackets Default for #name {
                fn default() -> Self {
                    unsafe { zeroed() }
                }
            }

            #[cfg(feature = "debug")]
            impl #lifetime_brackets fmtd::Debug for #name {
                fn fmt(&self, f: &mut fmtd::Formatter) -> fmtd::Result {
                    write!(f, #name_string)
                }
            }
        };

        octx.emit_tokens(code)
    }
}
