use crate::{
    Context,
    output::{CodeMap, Destination},
};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::{
        CommandItem, RequireLocation,
        function::{Command, CommandParam, Length},
    },
    name::{CommandName, TypeName, VariableName},
    rust::{RustTokens, RustTy},
};
use heck::ToSnekCase;
use indexmap::IndexMap;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::ffi::CString;
use syn::Ident;
use tracing::{debug, instrument, trace};

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
            Ty::ApiType(name) if name == TypeName::VK_DEVICE => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_COMMAND_BUFFER => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_QUEUE => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_INSTANCE => FunctionType::Instance,
            Ty::ApiType(name) if name == TypeName::VK_PHYSICAL_DEVICE => FunctionType::Instance,
            _ => FunctionType::Entry,
        }
    }

    fn table_name(self, dest: &Destination) -> Ident {
        match (self, dest.location) {
            (FunctionType::Static, _) => format_ident!("StaticFn"),
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

    fn table_field(self, dest: &Destination) -> Ident {
        match dest.location {
            RequireLocation::Core { major, minor } => match self {
                FunctionType::Static => format_ident!("static_fn"),
                FunctionType::Entry => format_ident!("entry_fn_{major}_{minor}"),
                FunctionType::Instance => format_ident!("instance_fn_{major}_{minor}"),
                FunctionType::Device => format_ident!("device_fn_{major}_{minor}"),
            },
            RequireLocation::Extension { .. } => format_ident!("fp"),
        }
    }

    fn loader_name(self) -> Ident {
        match self {
            FunctionType::Static | FunctionType::Entry => format_ident!("Entry"),
            FunctionType::Instance => format_ident!("Instance"),
            FunctionType::Device => format_ident!("Device"),
        }
    }
}

pub fn generate_code(ctx: &Context, codemap: &mut CodeMap) {
    debug!("generating loader code");

    #[derive(Default)]
    struct Table {
        fields: TokenStream,
        loaders: TokenStream,
        wrappers: TokenStream,
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
        for mut dest in Destination::all_locations(required_by) {
            let table_field = function_type.table_field(&dest);

            dest.reexport = false;
            let table = tables.entry((function_type, dest)).or_default();

            let field_name = format_ident!("{}", name.prefix_stripped().to_snek_case());
            let command_ty = ctx.command_tokens(name, true);
            table.fields.extend(quote! {
                pub #field_name: #command_ty,
            });

            if ![
                CommandName::VK_GET_INSTANCE_PROC_ADDR,
                CommandName::VK_GET_DEVICE_PROC_ADDR,
                CommandName::VK_ENUMERATE_INSTANCE_VERSION,
                CommandName::VK_CREATE_INSTANCE,
                CommandName::VK_CREATE_DEVICE,
            ]
            .contains(&name)
            {
                let code = wrapper(ctx, command, &field_name, table_field);
                table.wrappers.extend(code);
            }

            let panic_msg = format!("unable to load {}", name.original());
            let cstr = Literal::c_string(&CString::new(name.original()).unwrap());

            let params = command.params.iter().map(|param| {
                let ty = (param.decl.ty).to_rust().tokens(ctx, None);
                quote! { _: #ty }
            });

            let ret = command.return_type.as_ref().map(|ty| {
                let rust_ty = ty.to_rust().tokens(ctx, None);
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
    }

    for ((function_type, dest), table) in tables.into_iter() {
        let Table {
            fields,
            loaders,
            wrappers,
        } = table;

        let table_name = function_type.table_name(&dest);
        let table_code = quote! {
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

        let loader_name = function_type.loader_name();
        let loader_code = match (dest.location, function_type) {
            (RequireLocation::Core { major, minor }, _) => {
                let table_getter = matches!(
                    function_type,
                    FunctionType::Entry | FunctionType::Instance | FunctionType::Device
                )
                .then(|| {
                    let name = format_ident!("fp_v{major}_{minor}");
                    let table_field = function_type.table_field(&dest);
                    quote! {
                        #[inline]
                        pub fn #name(&self) -> &crate::#table_name {
                            &self.#table_field
                        }
                    }
                });

                let doc = dest.provided_by_doc_comment();
                Some(quote! {
                    #[doc = #doc]
                    impl crate::#loader_name {
                        #table_getter
                        #wrappers
                    }
                })
            }
            (RequireLocation::Extension { .. }, FunctionType::Instance) => Some(quote! {
                #[derive(Clone)]
                pub struct #loader_name {
                    pub(crate) fp: #table_name,
                    pub(crate) handle: crate::vk::Instance,
                }

                impl #loader_name {
                    pub fn load(entry: &crate::Entry, instance: &crate::Instance) -> Self {
                        let handle = instance.handle;
                        let fp = #table_name::load(|name| unsafe {
                            core::mem::transmute(entry.get_instance_proc_addr(handle, name.as_ptr()))
                        });
                        Self { handle, fp }
                    }

                    #[inline]
                    pub fn fp(&self) -> &#table_name {
                        &self.fp
                    }

                    #[inline]
                    pub fn instance(&self) -> crate::vk::Instance {
                        self.handle
                    }

                    #wrappers
                }
            }),
            (RequireLocation::Extension { .. }, FunctionType::Device) => Some(quote! {
                #[derive(Clone)]
                pub struct #loader_name {
                    pub(crate) fp: #table_name,
                    pub(crate) handle: crate::vk::Device,
                }

                impl #loader_name {
                    pub fn load(instance: &crate::Instance, device: &crate::Device) -> Self {
                        let handle = device.handle;
                        let fp = #table_name::load(|name| unsafe {
                            core::mem::transmute(instance.get_device_proc_addr(handle, name.as_ptr()))
                        });
                        Self { handle, fp }
                    }

                    #[inline]
                    pub fn fp(&self) -> &#table_name {
                        &self.fp
                    }

                    #[inline]
                    pub fn device(&self) -> crate::vk::Device {
                        self.handle
                    }

                    #wrappers
                }
            }),
            _ => None,
        };

        codemap.extend(CodeMap::new(dest, quote! { #table_code #loader_code }));
    }
}

#[instrument(skip(ctx))]
fn wrapper(ctx: &Context, command: &Command, name: &Ident, table_field: Ident) -> TokenStream {
    trace!("generating");

    // let mut a = HashSet::new();
    // for p in &command.params {
    //     if let Ty::Ptr(_, Mutability::Mut) = p.decl.ty
    //         && let Some(Length::DefinedByMember(m)) = p.length
    //     {
    //         a.insert(m);
    //     }
    // }

    // assert!(a.len() < 2, "{a:#?}");

    struct WrapperParam<'a> {
        param: &'a CommandParam,
        name: Ident,
        call_arg: TokenStream,
        public_type: Option<TokenStream>,
    }

    let mut wrapper_params: IndexMap<VariableName, WrapperParam> = (command.params.iter())
        .map(|param| {
            let stripped = crate::strip_leading_p(param.decl.name.original()).to_snek_case();
            let name = crate::escape_ident(&stripped);

            let wrapper = WrapperParam {
                param,
                call_arg: quote! { #name },
                name,
                public_type: None,
            };

            (param.decl.name, wrapper)
        })
        .collect();

    for wrapper in wrapper_params.values_mut() {
        let name = &wrapper.name;
        let public_type = match &wrapper.param.decl.ty {
            Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), Mutability::Not)
                if matches!(wrapper.param.length, Some(Length::NullTerminated)) =>
            {
                wrapper.call_arg = quote! { #name.map_or(core::ptr::null(), |s| s.as_ptr()) };
                Some(RustTy::Option(Box::new(RustTy::Ref(
                    Box::new(RustTy::CStr),
                    Mutability::Not,
                ))))
            }
            ty => Some(ty.to_rust()),
        };

        wrapper.public_type = public_type.map(|ty| ty.tokens(ctx, None));
    }

    let call_args = wrapper_params.values().map(|wrapper| &wrapper.call_arg);
    let public_params = wrapper_params.values().filter_map(|wrapper| {
        let name = &wrapper.name;
        wrapper.public_type.as_ref().map(|ty| quote! { #name: #ty })
    });

    let ret = command.return_type.as_ref().map(|ty| {
        let rust_ty = ty.to_rust().tokens(ctx, None);
        quote! { -> #rust_ty }
    });

    quote! {
        #[inline]
        pub unsafe fn #name(&self #( , #public_params )*) #ret {
            (self.#table_field.#name)( #( #call_args ),* )
        }
    }
}
