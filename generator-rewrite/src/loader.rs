use crate::{
    Context,
    output::{CodeMap, Destination},
};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::{
        CommandItem, RequireLocation, TypeItem,
        function::{Command, Length},
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

            let field_name = format_ident!("{}", name.prefix_and_tag_stripped().to_snek_case());
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

    match command.name.original() {
        "vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR"
        | "vkEnumeratePhysicalDeviceQueueFamilyPerformanceCountersByRegionARM"
        | "vkGetPipelineBinaryDataKHR"
        | "vkGetEncodedVideoSessionParametersKHR" => {
            // todo
            return quote! {};
        }

        _ => (),
    }

    fn param_ident(name: VariableName) -> Ident {
        let stripped = crate::strip_leading_p(name.original()).to_snek_case();
        crate::escape_ident(&stripped)
    }

    struct WrapperParam {
        call_arg: TokenStream,
        /// if this is `None`, then the parameter will not be exposed
        public_type: Option<RustTy>,
    }

    let mut wrapper_params: IndexMap<VariableName, Option<WrapperParam>> = (command.params.iter())
        .map(|param| (param.decl.name, None))
        .collect();
    let mut length_calculations: IndexMap<VariableName, Vec<TokenStream>> = IndexMap::new();

    enum MultiCallKind {
        ReadIntoUninitializedVector,
        SeperateLenMethod,
    }

    struct MultiCallLength {
        kind: MultiCallKind,
        count: VariableName,
        data: VariableName,
        element: &'static Ty,
    }

    let mut multi_call_length = None;
    for (i, param) in command.params.iter().enumerate() {
        let name = param_ident(param.decl.name);

        let mut call_arg;
        let public_type;
        match &param.decl.ty {
            Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), Mutability::Not)
                if matches!(param.length, Some(Length::NullTerminated)) =>
            {
                call_arg = quote! { #name.map_or(core::ptr::null(), |s| s.as_ptr()) };
                public_type = Some(RustTy::Custom {
                    custom_type: quote! { Option },
                    generic_args: vec![RustTy::Ref(
                        Box::new(RustTy::Custom {
                            custom_type: quote! { core::ffi::CStr },
                            generic_args: vec![],
                        }),
                        Mutability::Not,
                    )],
                });
            }
            Ty::Ptr(element, Mutability::Mut)
                if let Some(Length::DefinedByParam(length_param)) = param.length
                    && let Ty::Ptr(.., Mutability::Mut) = {
                        let length_param = (command.params.iter())
                            .find(|other_param| other_param.decl.name == length_param)
                            .unwrap();
                        &length_param.decl.ty
                    } =>
            {
                let length_param_name = param_ident(length_param);
                let length_wrapper_param = &mut wrapper_params[&length_param];

                assert!(
                    length_wrapper_param.is_none(),
                    "only one multi call length parameter is supported"
                );

                let kind = if let Ty::ApiType(element_type) = element
                    && let TypeItem::Struct(structure) =
                        ctx.items.types[element_type].resolve_alias(ctx.items)
                    && let Some(Mutability::Mut) = structure.p_next
                {
                    call_arg = quote! { #name.as_mut_ptr() };
                    public_type = Some(RustTy::Slice(
                        Box::new(element.to_rust()),
                        Mutability::Mut,
                        None,
                    ));

                    *length_wrapper_param = Some(WrapperParam {
                        call_arg: quote! { &mut #length_param_name },
                        public_type: None,
                    });

                    MultiCallKind::SeperateLenMethod
                } else {
                    call_arg = quote! { #name };
                    public_type = None;

                    *length_wrapper_param = Some(WrapperParam {
                        call_arg: quote! { #length_param_name },
                        public_type: None,
                    });

                    MultiCallKind::ReadIntoUninitializedVector
                };

                multi_call_length = Some(MultiCallLength {
                    kind,
                    count: length_param,
                    data: param.decl.name,
                    element,
                });
            }
            &Ty::Ptr(element, mutability)
                if let Some(Length::DefinedByParam(length_param)) = param.length =>
            {
                let mut element = element.clone();
                call_arg = match mutability {
                    Mutability::Mut => quote! { #name.as_mut_ptr() },
                    Mutability::Not => quote! { #name.as_ptr() },
                };

                if let Ty::CPrimary(CPrimaryType::Void) = element {
                    element = Ty::CPrimary(CPrimaryType::UInt8);
                    call_arg = quote! { #call_arg.cast() };
                }

                public_type = Some(RustTy::Slice(Box::new(element.to_rust()), mutability, None));
                wrapper_params[&length_param] = Some(WrapperParam {
                    call_arg: quote! { #name.len() as _ },
                    public_type: None,
                });

                length_calculations
                    .entry(length_param)
                    .or_default()
                    .push(quote! { #name.len() });
            }
            &Ty::ApiType(type_name)
                if type_name == TypeName::VK_INSTANCE || type_name == TypeName::VK_DEVICE =>
            {
                call_arg = quote! { self.handle };
                public_type = None;
            }
            _ => continue,
        };

        *wrapper_params.values_mut().nth(i).unwrap() = Some(WrapperParam {
            call_arg,
            public_type,
        })
    }

    let call_args_map: IndexMap<VariableName, TokenStream> = wrapper_params
        .iter()
        .map(|(&name, wrapper)| {
            let value = match wrapper {
                Some(wrapper) => wrapper.call_arg.clone(),
                None => {
                    let name = param_ident(name);
                    quote! { #name }
                }
            };

            (name, value)
        })
        .collect();

    let public_params_map: IndexMap<VariableName, TokenStream> = command
        .params
        .iter()
        .enumerate()
        .filter_map(|(i, param)| {
            let ty = if let Some(wrapper) = wrapper_params.values().nth(i).unwrap() {
                &wrapper.public_type
            } else if let Ty::Ptr(to, mutability) = param.decl.ty {
                &Some(RustTy::Ref(Box::new(to.to_rust()), mutability))
            } else {
                &Some(param.decl.ty.to_rust())
            };

            ty.as_ref().map(|ty| {
                let name = param_ident(param.decl.name);
                let ty_tokens = ty.tokens(ctx, None);
                (param.decl.name, quote! { #name: #ty_tokens })
            })
        })
        .collect();

    let call_args = call_args_map.values();
    let mut content = quote! { (self.#table_field.#name)( #( #call_args ),* ) };
    for calculation in length_calculations.values() {
        for [a, b] in calculation.array_windows() {
            content = quote! {
                assert_eq!(#a, #b);
                #content
            };
        }
    }

    let doc = command.name.original(); // TODO: improve

    let mut len_method = None;
    let returns_result = matches!(command.return_type, Some(Ty::ApiType(TypeName::VK_RESULT)));
    let ret_ty;
    if let Some(multi_call_length) = multi_call_length {
        let count = param_ident(multi_call_length.count);
        let data = param_ident(multi_call_length.data);
        match multi_call_length.kind {
            MultiCallKind::ReadIntoUninitializedVector => {
                let mut ret_ty_value = RustTy::Custom {
                    custom_type: quote! { Vec },
                    generic_args: vec![multi_call_length.element.to_rust()],
                };

                if returns_result {
                    content =
                        quote! { crate::read_into_uninitialized_vector(|#count, #data| #content) };
                    ret_ty_value = RustTy::Custom {
                        custom_type: quote! { crate::VkResult },
                        generic_args: vec![ret_ty_value],
                    };
                } else {
                    content = quote! {
                        crate::read_into_uninitialized_vector(|#count, #data| {
                            #content;
                            crate::vk::Result::SUCCESS
                        }).unwrap()
                    };
                }

                ret_ty = Some(ret_ty_value);
            }
            MultiCallKind::SeperateLenMethod => {
                if returns_result {
                    content = quote! { #content.result()? };
                }

                content = quote! {
                    let mut #count = #data.len() as _;
                    #content;
                    assert_eq!(#count as usize, #data.len());
                };

                if returns_result {
                    content = quote! { #content Ok(()) }
                }

                ret_ty = returns_result.then(|| RustTy::Custom {
                    custom_type: quote! { crate::VkResult },
                    generic_args: vec![RustTy::Unit],
                });

                // TODO: refactor this code. it's ugly
                let mut public_params_map = public_params_map.clone();
                public_params_map.shift_remove(&multi_call_length.data);
                let mut call_args_map = call_args_map.clone();
                call_args_map[&multi_call_length.count] = quote! { #count.as_mut_ptr() };
                call_args_map[&multi_call_length.data] = quote! { core::ptr::null_mut() };

                let name_len = format_ident!("{name}_len");
                let public_params = public_params_map.values();
                let call_args = call_args_map.values();
                let mut content = quote! { (self.#table_field.#name)( #( #call_args ),* ) };
                let ret = if returns_result {
                    content = quote! {
                        #content
                            .assume_init_on_success(#count)
                            .map(|c| c as usize)
                    };

                    quote! { -> crate::VkResult<usize> }
                } else {
                    content = quote! {
                        #content;
                        #count.assume_init() as usize
                    };

                    quote! { -> usize }
                };

                len_method = Some(quote! {
                    #[doc = #doc]
                    #[inline]
                    pub unsafe fn #name_len(&self #( , #public_params )*) #ret {
                        let mut #count = core::mem::MaybeUninit::uninit();
                        #content
                    }
                });
            }
        }
    } else if returns_result {
        content = quote! { #content.result() };
        ret_ty = Some(RustTy::Custom {
            custom_type: quote! { crate::VkResult },
            generic_args: vec![RustTy::Unit],
        });
    } else {
        ret_ty = command.return_type.as_ref().map(|ty| ty.to_rust());
    };

    let ret = ret_ty.map(|ty| {
        let tokens = ty.tokens(ctx, None);
        quote! { -> #tokens }
    });

    let public_params = public_params_map.values();
    quote! {
        #len_method

        #[doc = #doc]
        #[inline]
        pub unsafe fn #name(&self #( , #public_params )*) #ret {
            #content
        }
    }
}
