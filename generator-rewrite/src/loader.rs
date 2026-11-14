use crate::{
    Context, Function, Safety,
    output::{CodeMap, GlobExport, OutputContext, OutputLocation},
};
use analysis::{
    decl::{CPrimaryType, Mutability, Ty},
    item::{
        CommandItem, RequireLocation, TypeItem,
        function::{Command, CommandParam, Length},
    },
    name::{CommandName, TypeName, VariableName},
    rust::{RustDecl, RustTokens, RustTy},
};
use heck::ToSnekCase;
use indexmap::IndexMap;
use proc_macro2::{Literal, TokenStream};
use quote::{ToTokens, format_ident, quote};
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

        let (_name, first_param) = command.params.first().expect("cmds should have params");
        match first_param.decl.ty {
            Ty::ApiType(name) if name == TypeName::VK_DEVICE => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_COMMAND_BUFFER => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_QUEUE => FunctionType::Device,
            Ty::ApiType(name) if name == TypeName::VK_INSTANCE => FunctionType::Instance,
            Ty::ApiType(name) if name == TypeName::VK_PHYSICAL_DEVICE => FunctionType::Instance,
            _ => FunctionType::Entry,
        }
    }

    fn table_name(self, require: RequireLocation) -> Ident {
        match (self, require) {
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

    fn table_field(self, require: RequireLocation) -> Ident {
        match require {
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

    let mut tables: IndexMap<(FunctionType, OutputLocation), Table> = Default::default();
    for command_item in ctx.items.commands.values() {
        let (name, required_by, command) = match command_item {
            CommandItem::Alias(alias) => match &ctx.items.commands[&alias.target] {
                CommandItem::Alias(..) => unreachable!(),
                CommandItem::Command(command) => (alias.name, alias.required_by, command),
            },
            CommandItem::Command(command) => (command.name, command.required_by, command),
        };
        let resolved_name = command.name;

        let function_type = FunctionType::of_command(command);
        for output in OutputLocation::all_of(required_by, GlobExport::No) {
            let octx = codemap.octx(output, ctx);

            let table_field = function_type.table_field(output.require);
            let table = tables.entry((function_type, output)).or_default();

            let wrapper_name = format_ident!("{}", name.prefix_and_tag_stripped().to_snek_case());
            let command_ty = octx.command_tokens(name, true);
            table.fields.extend(quote! {
                pub #wrapper_name: #command_ty,
            });

            let methods = ctx.wrapper_rules.get(resolved_name).map_or_else(
                || itertools::Either::Left(wrapper(&octx, command, &wrapper_name)),
                itertools::Either::Right,
            );

            let methods_code = methods.iter().map(
                |Function {
                     attributes,
                     safety,
                     name_override,
                     generics,
                     inputs,
                     output,
                     content,
                 }| {
                    let safety = match safety {
                        Safety::Default | Safety::Unsafe => quote! { unsafe },
                        Safety::Safe => quote! {},
                    };

                    let description = match function_type {
                        FunctionType::Static => "Static Function",
                        FunctionType::Entry => "Entry Function",
                        FunctionType::Instance => "Instance Function",
                        FunctionType::Device => "Device Function",
                    };

                    let name = name_override.as_ref().unwrap_or(&wrapper_name);
                    let manual_page =
                        crate::manual_page_doc_comment(command.name.original(), description);
                    let content = crate::replace_ident(
                        content.clone(),
                        &format_ident!("__table"),
                        &table_field,
                    );

                    quote! {
                        #[doc = #manual_page]
                        #attributes
                        pub #safety fn #name #generics(#inputs) #output { #content }
                    }
                },
            );

            table.wrappers.extend(quote! { #(#methods_code)* });

            let panic_msg = format!("unable to load {}", name.original());
            let cstr = Literal::c_string(&CString::new(name.original()).unwrap());

            let params = command.params.values().map(|param| {
                let ty = (param.decl.ty).to_rust().tokens(&octx, None);
                quote! { _: #ty }
            });

            let ret = command.return_type.as_ref().map(|ty| {
                let rust_ty = ty.to_rust().tokens(&octx, None);
                quote! { -> #rust_ty }
            });

            table.loaders.extend(quote! {
                #wrapper_name: unsafe {
                    unsafe extern "system" fn #wrapper_name( #( #params ),* ) #ret {
                        panic!(#panic_msg)
                    }

                    let val = _f(#cstr);
                    if val.is_null() {
                        #wrapper_name
                    } else {
                        transmute(val)
                    }
                },
            });
        }
    }

    for ((function_type, output), table) in tables.into_iter() {
        let octx = codemap.octx(output, ctx);
        let Table {
            fields,
            loaders,
            wrappers,
        } = table;

        let table_name = function_type.table_name(output.require);
        let table_code = quote! {
            #[derive(Clone)]
            pub struct #table_name {
                #fields
            }

            unsafe impl Send for #table_name {}
            unsafe impl Sync for #table_name {}

            impl #table_name {
                pub fn load<F: FnMut(&CStr) -> *const ffi::c_void>(mut f: F) -> Self {
                    Self::load_erased(&mut f)
                }

                fn load_erased(_f: &mut dyn FnMut(&CStr) -> *const ffi::c_void) -> Self {
                    Self { #loaders }
                }
            }
        };

        let loader_name = function_type.loader_name();
        let loader_code = match (output.require, function_type) {
            (RequireLocation::Core { major, minor }, _) => {
                let table_getter = matches!(
                    function_type,
                    FunctionType::Entry | FunctionType::Instance | FunctionType::Device
                )
                .then(|| {
                    let name = format_ident!("fp_v{major}_{minor}");
                    let table_field = function_type.table_field(output.require);
                    quote! {
                        #[inline]
                        pub fn #name(&self) -> &crate::#table_name {
                            &self.#table_field
                        }
                    }
                });

                let doc = output.provided_by_doc_comment();
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
                    pub(crate) handle: vk::Instance,
                }

                impl #loader_name {
                    pub fn load(entry: &crate::Entry, instance: &crate::Instance) -> Self {
                        let handle = instance.handle;
                        let fp = #table_name::load(|name| unsafe {
                            transmute(entry.get_instance_proc_addr(handle, name.as_ptr()))
                        });
                        Self { handle, fp }
                    }

                    #[inline]
                    pub fn fp(&self) -> &#table_name {
                        &self.fp
                    }

                    #[inline]
                    pub fn instance(&self) -> vk::Instance {
                        self.handle
                    }

                    #wrappers
                }
            }),
            (RequireLocation::Extension { .. }, FunctionType::Device) => Some(quote! {
                #[derive(Clone)]
                pub struct #loader_name {
                    pub(crate) fp: #table_name,
                    pub(crate) handle: vk::Device,
                }

                impl #loader_name {
                    pub fn load(instance: &crate::Instance, device: &crate::Device) -> Self {
                        let handle = device.handle;
                        let fp = #table_name::load(|name| unsafe {
                            transmute(instance.get_device_proc_addr(handle, name.as_ptr()))
                        });
                        Self { handle, fp }
                    }

                    #[inline]
                    pub fn fp(&self) -> &#table_name {
                        &self.fp
                    }

                    #[inline]
                    pub fn device(&self) -> vk::Device {
                        self.handle
                    }

                    #wrappers
                }
            }),
            _ => None,
        };

        octx.emit_tokens(quote! { #table_code #loader_code });
    }
}

/// the following acts a rough guidance for fixing this wrapper code
///
/// A. first argument is passed through self.handle into the call
/// B. last argument is AllocationCallbacks, passed as match ... into the call
/// C. return nothing
/// D. instantiate 1 handle/DeviceSize/non-pnext/flags through MaybeUninit, pass along everything else
///      call .assume_init() at end and return directly
/// E. slice of (pnext?) structs is passed as .len() and .as_ptr() into the call (cause u32,*const)
/// F. vk::Result return gets turned into `VkResult<()>`, with .result() called on the function directly
/// G. instantiate 1 handle/void-pointer through MaybeUninit, pass along everything else
///      call .assume_init_on_success() at end and return inside VkResult<>
/// H. read_into_uninitialized_vector is used to return Vec<> of non-pnext, (cause *mut u32,*mut)
///      (but it may return nothing so we don't VkResult<> and instead ;vk::Result::SUCCESS and unwrap)
/// I. read_into_uninitialized_vector is used to return VkResult<Vec<u8>>, (cause *mut usize,*mut c_void)
///      and we tell the closure that the type is actually u8
/// J. *const and *mut arguments have the same u32 length,
///      therefore we construct `Result<Vec<HANDLE>, (Vec<HANDLE>, vk::Result)>`
/// K. `*const [f32; _]` is taken as &[ffi::c_float; 4 as _]
/// L. two or more *const arrays have the same size, and are asserted to have the same length
///      and we choose the first one in the call
/// M. similar to H, but now there's a seperate _len method because it is pnext
/// N. we wanted to return a struct like in D but it's pnext so we do `out: &mut`
/// O. overridden because fancy
/// P. Handle generics through objecttype="objectType"
/// Q. similiar to M, but now there's multiple slices filled out of the same _len method
/// R. similiar to N, but it's marked optional, so we actually do `out: Option<&mut`
/// S. similiar to D, but it's a c_void so we actually don't return anything...
///
/// A B C   vkDestroyDevice
/// A D     vkGetDeviceQueue/vkGetDeviceMemoryCommitment/vkGetBufferMemoryRequirements/vkGetDeviceGroupPeerMemoryFeatures
/// E F     vkQueueSubmit
/// F       vkQueueWaitIdle
/// A F     vkDeviceWaitIdle/vkBindBufferMemory
/// B G     vkAllocateMemory
/// B C     vkFreeMemory
/// G       vkMapMemory
/// A C     vkUnmapMemory
/// A E F   vkFlushMappedMemoryRanges
/// A H     vkGetImageSparseMemoryRequirements
/// A I     vkGetPipelineCacheData
/// A J     vkCreateGraphicsPipelines/vkCreateShadersEXT (search for ">, (Vec<")
/// C K     vkCmdSetBlendConstants
/// C L     vkCmdBindVertexBuffers
/// A M     vkGetImageSparseMemoryRequirements2
/// A N     vkGetImageMemoryRequirements2
/// O       vkGetQueryPoolResults
/// A P     vkSetPrivateDataEXT
/// Q       vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR
/// A M N   vkGetPipelineBinaryDataKHR
/// A M N R vkGetEncodedVideoSessionParametersKHR
/// A F R   vkGetDeviceFaultInfoEXT
/// A S     vkGetLatencyTimingsLegacyNV
struct _A;

#[derive(Debug)]
struct WrapperOutputParam<'a> {
    param: &'a CommandParam,
    array: bool,
    tagged: bool,
}

impl<'a> WrapperOutputParam<'a> {}

#[derive(Debug)]
struct WrapperOutput<'a> {
    result: bool,
    params: Vec<WrapperOutputParam<'a>>,
}

impl<'a> WrapperOutput<'a> {
    fn new(ctx: &Context, command: &'a Command) -> WrapperOutput<'a> {
        let result =
            matches!(command.return_type, Some(Ty::ApiType(name)) if name == TypeName::VK_RESULT);

        let params: Vec<_> = command
            .params
            .values()
            .filter(|param| {
                param.length_of.is_empty() && matches!(&param.decl.ty, Ty::Ptr(_, Mutability::Mut))
            })
            .filter_map(|param| {
                if param.length_of.is_empty()
                    && let Ty::Ptr(to, Mutability::Mut) = &param.decl.ty
                {
                    let mut array = false;
                    if param.optional == [true]
                        && let Some(Length::DefinedByParam(length_param)) = &param.length
                        && command.params[length_param].optional == [false, true]
                    {
                        array = true;
                    }

                    let mut tagged = false;
                    if let Ty::ApiType(name) = to
                        && let TypeItem::Struct(struct_) = ctx.resolve_type(*name)
                        && struct_.p_next.is_some()
                    {
                        tagged = true;
                    }

                    return Some(WrapperOutputParam {
                        param,
                        array,
                        tagged,
                    });
                }

                None
            })
            .collect();

        WrapperOutput { result, params }
    }
}

fn param_ident(name: VariableName) -> Ident {
    let stripped = crate::strip_leading_p(name.original()).to_snek_case();
    crate::escape_ident(&stripped)
}

#[derive(Debug)]
struct WrapperParam {
    name: VariableName,
    ty: RustTy,
}

#[derive(Debug)]
struct WrapperCallArg {
    param: Option<WrapperParam>,
    value: TokenStream,
}

impl WrapperCallArg {
    fn new(param: &CommandParam, _wrapper_output: &WrapperOutput) -> WrapperCallArg {
        if let Some(length_of) = param.length_of.first() {
            let length_of = param_ident(*length_of);
            return WrapperCallArg {
                param: None,
                value: quote! { #length_of.len() as _ },
            };
        }

        let name = param.decl.name;
        let ident = param_ident(name);
        match (&param.decl.ty, &param.length) {
            (
                Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), Mutability::Not),
                Some(Length::NullTerminated),
            ) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Custom {
                        custom_type: quote! { Option },
                        generic_args: vec![RustTy::Ref(
                            Box::new(RustTy::Custom {
                                custom_type: quote! { CStr },
                                generic_args: vec![],
                            }),
                            Mutability::Not,
                        )],
                    },
                }),
                value: quote! { #ident.map_or(ptr::null(), |s| s.as_ptr()) },
            },
            (&Ty::ApiType(type_name), _) if type_name == TypeName::VK_BOOL32 => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Bool,
                }),
                value: quote! { #ident as _ },
            },
            (&Ty::ApiType(type_name), _)
                if type_name == TypeName::VK_INSTANCE || type_name == TypeName::VK_DEVICE =>
            {
                WrapperCallArg {
                    param: None,
                    value: quote! { self.handle },
                }
            }
            (Ty::Ptr(to, Mutability::Not), None) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Ref(Box::new(to.to_rust()), Mutability::Not),
                }),
                value: quote! { #ident },
            },
            (Ty::Ptr(to, Mutability::Not), Some(Length::DefinedByParam(_))) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::ArrayRef(Box::new(to.to_rust()), Mutability::Not, None),
                }),
                value: quote! { #ident.as_ptr() },
            },
            _ => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: param.decl.ty.to_rust(),
                }),
                value: quote! { #ident },
            },
        }
    }
}

fn wrapper(octx: &OutputContext, command: &Command, name: &Ident) -> Vec<Function> {
    let wrapper_output = WrapperOutput::new(&octx.ctx(), command);

    let call_args: Vec<_> = command
        .params
        .values()
        .map(|param| WrapperCallArg::new(param, &wrapper_output))
        .collect();

    let param_code = call_args.iter().filter_map(|call_arg| {
        call_arg.param.as_ref().map(|wrapper_param| {
            let ident = param_ident(wrapper_param.name);
            let ty = wrapper_param.ty.tokens(octx, None);
            quote! { #ident: #ty }
        })
    });
    let inputs = quote! { &self, #( #param_code ),* };

    let arg_code = call_args.iter().map(|param| &param.value);
    let call = quote! { (self.__table.#name)( #( #arg_code ),* ) };

    match wrapper_output.params.as_slice() {
        [] => {
            vec![Function {
                inputs,
                output: if wrapper_output.result {
                    quote! { -> VkResult<()> }
                } else if let Some(return_ty) = &command.return_type {
                    let return_ty = return_ty.to_rust().tokens(octx, None);
                    quote! { -> #return_ty }
                } else {
                    quote! {}
                },
                content: if wrapper_output.result {
                    quote! { #call.result() }
                } else {
                    quote! { #call }
                },
                ..Default::default()
            }]
        }
        _ => vec![],
    }
}
