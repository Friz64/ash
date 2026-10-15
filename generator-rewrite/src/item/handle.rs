use super::{Code, Context};
use crate::{
    Import,
    output::{CodeMap, GlobExport, OutputLocation},
};
use analysis::{item::handle::Handle, name::TypeName, rust::RustTokens};
use quote::quote;
use tracing::{instrument, trace};

impl Code for Handle {
    #[instrument(skip(ctx, codemap))]
    fn code(&self, ctx: &Context, codemap: &mut CodeMap) {
        trace!("generating");
        let output = OutputLocation::primary_location(self.required_by, GlobExport::Yes);
        let octx = codemap.octx(output, ctx);
        let name = octx.type_tokens(self.name, false, None);
        let ty = octx.enumerator_tokens(self.object_type, TypeName::VK_OBJECT_TYPE, true);

        octx.insert_import(Import::CrateVk);
        octx.insert_import(Import::CrateHandle);
        octx.insert_import(Import::CoreFmt);
        octx.insert_import(Import::DebugCoreFmt);
        let code = if self.dispatchable {
            octx.insert_import(Import::CorePtr);
            quote! {
                #[repr(transparent)]
                #[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash)]
                pub struct #name(*mut u8);

                impl Default for #name {
                    fn default() -> Self {
                        Self::null()
                    }
                }

                impl Handle for #name {
                    const TYPE: vk::ObjectType = #ty;

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
                        Self(ptr::null_mut())
                    }
                }

                impl fmt::Pointer for #name {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        fmt::Pointer::fmt(&self.0, f)
                    }
                }

                #[cfg(feature = "debug")]
                impl fmtd::Debug for #name {
                    fn fmt(&self, f: &mut fmtd::Formatter) -> fmtd::Result {
                        fmtd::Debug::fmt(&self.0, f)
                    }
                }
            }
        } else {
            quote! {
                #[repr(transparent)]
                #[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Default)]
                pub struct #name(u64);

                impl Handle for #name {
                    const TYPE: vk::ObjectType = #ty;

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

                impl fmt::Pointer for #name {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        write!(f, "0x{:x}", self.0)
                    }
                }

                #[cfg(feature = "debug")]
                impl fmtd::Debug for #name {
                    fn fmt(&self, f: &mut fmtd::Formatter) -> fmtd::Result {
                        write!(f, "0x{:x}", self.0)
                    }
                }
            }
        };

        octx.insert_tokens(code)
    }
}
