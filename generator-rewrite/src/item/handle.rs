use super::{Code, Context};
use crate::output::{CodeMap, Destination};
use analysis::{item::handle::Handle, lifetime::Lifetime, name::TypeName, rust::RustTokens};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Handle {
    #[instrument(skip(ctx))]
    fn code(&self, ctx: &Context) -> CodeMap {
        trace!("generating");
        let name = ctx.type_tokens(self.name, false, &Lifetime::placeholder());
        let ty = ctx.enumerator_tokens(self.object_type, TypeName::VK_OBJECT_TYPE, true);

        let code = if self.dispatchable {
            quote! {
                #[repr(transparent)]
                #[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash)]
                pub struct #name(*mut u8);

                impl Default for #name {
                    fn default() -> Self {
                        Self::null()
                    }
                }

                impl crate::Handle for #name {
                    const TYPE: crate::vk::ObjectType = #ty;

                    fn as_raw(self) -> u64 {
                        self.0 as u64
                    }

                    fn from_raw(x: u64) -> Self {
                        Self(x as _)
                    }
                }

                unsafe impl Send for #name {}
                unsafe impl Sync for #name {}

                impl #name {
                    pub const fn null() -> Self {
                        Self(::core::ptr::null_mut())
                    }
                }

                impl core::fmt::Pointer for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        core::fmt::Pointer::fmt(&self.0, f)
                    }
                }

                impl core::fmt::Debug for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        core::fmt::Debug::fmt(&self.0, f)
                    }
                }
            }
        } else {
            quote! {
                #[repr(transparent)]
                #[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Default)]
                pub struct #name(u64);

                impl crate::Handle for #name {
                    const TYPE: crate::vk::ObjectType = #ty;

                    fn as_raw(self) -> u64 {
                        self.0
                    }

                    fn from_raw(x: u64) -> Self {
                        Self(x)
                    }
                }

                impl #name {
                    pub const fn null() -> Self {
                        Self(0)
                    }
                }

                impl core::fmt::Pointer for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        write!(f, "0x{:x}", self.0)
                    }
                }

                impl core::fmt::Debug for #name {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        write!(f, "0x{:x}", self.0)
                    }
                }
            }
        };

        CodeMap::new(Destination::new(self.required_by), code)
    }
}
