use crate::{
    Context,
    output::{CodeMap, Destination},
};
use analysis::{
    decl::Ty,
    item::{CommandItem, RequireLocation, function::Command},
    lifetime::Lifetime,
    name::{CommandName, TypeName},
    to_rust::RustTranslator,
};
use heck::ToSnekCase;
use indexmap::IndexMap;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::ffi::CString;
use syn::Ident;
use tracing::debug;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum FunctionType {
    Static,
    Entry,
    Instance,
    Device,
}

impl FunctionType {
    fn of_command(command: &Command) -> FunctionType {
        if command.name == CommandName::VK_GET_INSTANCE_PROC_ADDR {
            return FunctionType::Static;
        } else if command.name == CommandName::VK_GET_DEVICE_PROC_ADDR {
            return FunctionType::Instance;
        }

        let first_param = command.params.first().expect("cmds should have params");
        match first_param.decl.ty {
            Ty::SpecType(name) if name == TypeName::VK_DEVICE => FunctionType::Device,
            Ty::SpecType(name) if name == TypeName::VK_COMMAND_BUFFER => FunctionType::Device,
            Ty::SpecType(name) if name == TypeName::VK_QUEUE => FunctionType::Device,
            Ty::SpecType(name) if name == TypeName::VK_INSTANCE => FunctionType::Instance,
            Ty::SpecType(name) if name == TypeName::VK_PHYSICAL_DEVICE => FunctionType::Instance,
            _ => FunctionType::Entry,
        }
    }

    fn table_name(self, dest: Destination) -> Ident {
        match (self, dest.location) {
            (FunctionType::Static, ..) => format_ident!("StaticFn"),
            (FunctionType::Entry, RequireLocation::Core { major, minor }) => {
                format_ident!("EntryFnV{major}_{minor}")
            }
            (FunctionType::Entry, RequireLocation::Extension { .. }) => format_ident!("EntryFn"),
            (FunctionType::Instance, RequireLocation::Core { major, minor }) => {
                format_ident!("InstanceFnV{major}_{minor}")
            }
            (FunctionType::Instance, RequireLocation::Extension { .. }) => {
                format_ident!("InstanceFn")
            }
            (FunctionType::Device, RequireLocation::Core { major, minor }) => {
                format_ident!("DeviceFnV{major}_{minor}")
            }
            (FunctionType::Device, RequireLocation::Extension { .. }) => {
                format_ident!("DeviceFn")
            }
        }
    }
}

pub fn generate_code(ctx: &Context, codemap: &mut CodeMap) {
    debug!("generating loader code");

    #[derive(Default)]
    struct Table {
        fields: TokenStream,
        loaders: TokenStream,
    }

    let mut tables: IndexMap<(FunctionType, Destination), Table> = Default::default();
    for command_item in ctx.items.commands.values() {
        let (name, required_by, command) = match command_item {
            CommandItem::Alias(alias) => match &ctx.items.commands[&alias.alias] {
                CommandItem::Alias(..) => unreachable!(),
                CommandItem::Command(command) => (alias.name, alias.required_by, command),
            },
            CommandItem::Command(command) => (command.name, command.required_by, command),
        };

        let function_type = FunctionType::of_command(command);
        let mut dest = Destination::new(required_by);
        dest.reexport = false;
        let table = tables.entry((function_type, dest)).or_default();

        let field_name = format_ident!("{}", name.prefix_trimmed().to_snek_case());
        let command_ty = ctx.command_to_rust(name, true);
        table.fields.extend(quote! {
            pub #field_name: #command_ty,
        });

        let panic_msg = format!("unable to load {}", name.original());
        let cstr = Literal::c_string(&CString::new(name.original()).unwrap());

        let params = command.params.iter().map(|param| {
            let ty = param.decl.ty.to_rust(ctx, &Lifetime::placeholder());
            quote! { _: #ty }
        });

        let ret = command.return_type.as_ref().map(|ty| {
            let rust_ty = ty.to_rust(ctx, &Lifetime::placeholder());
            quote! { -> #rust_ty }
        });

        table.loaders.extend(quote! {
            #field_name: unsafe {
                unsafe extern "system" fn #field_name( #( #params ),* ) #ret {
                    panic!(#panic_msg)
                }

                let val = _f(#cstr);
                if val.is_null() {
                    #field_name
                } else {
                    ::core::mem::transmute(val)
                }
            },
        });
    }

    for (&(function_type, dest), Table { fields, loaders }) in tables.iter() {
        let table_name = function_type.table_name(dest);
        let code = quote! {
            #[derive(Clone)]
            pub struct #table_name {
                #fields
            }

            unsafe impl Send for #table_name {}
            unsafe impl Sync for #table_name {}

            impl #table_name {
                pub fn load<F: FnMut(&::core::ffi::CStr) -> *const ::core::ffi::c_void>(mut f: F) -> Self {
                    Self::load_erased(&mut f)
                }

                fn load_erased(_f: &mut dyn FnMut(&::core::ffi::CStr) -> *const ::core::ffi::c_void) -> Self {
                    Self { #loaders }
                }
            }
        };

        codemap.extend(CodeMap::new(dest, code));
    }
}
