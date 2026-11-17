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
    rust::{RustTokens, RustTy},
};
use heck::ToSnekCase;
use indexmap::IndexMap;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use std::{ffi::CString, ptr};
use syn::Ident;
use tracing::{debug, trace};

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

            trace!(?command, "generating");
            let methods = ctx.wrapper_rules.get(resolved_name).map_or_else(
                || itertools::Either::Left(wrapper(&octx, command, &wrapper_name)),
                itertools::Either::Right,
            );

            let methods_code = methods.iter().map(
                |Function {
                     attributes,
                     safety,
                     name_override,
                     generic_params,
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

                    let generics = generic_params.as_ref().map(|params| quote! { <#params> });
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
                let ty = (param.decl.ty).to_rust().tokens(&*octx, None);
                quote! { _: #ty }
            });

            let ret = command.return_type.as_ref().map(|ty| {
                let rust_ty = ty.to_rust().tokens(&*octx, None);
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
enum WrapperOutputKind {
    UntaggedSingle,
    UntaggedArrayWithInput { input: VariableName },
    UntaggedArray,
    TaggedSingle,
    TaggedArray,
}

#[derive(Debug)]
struct WrapperOutput<'a> {
    kind: WrapperOutputKind,
    param: &'a CommandParam,
    inner_ty: &'static Ty,
}

impl<'a> WrapperOutput<'a> {
    fn list(ctx: &Context, command: &'a Command) -> Vec<WrapperOutput<'a>> {
        command
            .params
            .values()
            .filter(|param| {
                param.length_of.is_empty() && matches!(&param.decl.ty, Ty::Ptr(_, Mutability::Mut))
            })
            .filter_map(|param| {
                let mut array = false;
                let mut output_array = false;
                let mut array_input = None;
                if let Some(Length::DefinedByParam(length_param)) = param.length {
                    array = true;
                    output_array = matches!(
                        command.params[&length_param].decl.ty,
                        Ty::Ptr(.., Mutability::Mut)
                    );

                    let all_array_params = &command.params[&length_param].length_of;
                    array_input = all_array_params.iter().find(|&array_param| {
                        let array_param = &command.params[array_param];
                        matches!(array_param.decl.ty, Ty::Ptr(.., Mutability::Not))
                    });
                }

                if let Ty::Ptr(Ty::CPrimary(CPrimaryType::Void), Mutability::Mut) = &param.decl.ty
                    && !output_array
                {
                    return None;
                }

                if let Ty::Ptr(to, Mutability::Mut) = &param.decl.ty {
                    let mut tagged = false;
                    if let Ty::ApiType(name) = to
                        && let TypeItem::Struct(struct_) = ctx.resolve_type(*name)
                        && struct_.p_next.is_some()
                    {
                        tagged = true;
                    }

                    return Some(WrapperOutput {
                        kind: match (array, tagged) {
                            (false, false) => WrapperOutputKind::UntaggedSingle,
                            (true, false) if let Some(&input) = array_input => {
                                WrapperOutputKind::UntaggedArrayWithInput { input }
                            }
                            (true, false) => WrapperOutputKind::UntaggedArray,
                            (false, true) => WrapperOutputKind::TaggedSingle,
                            (true, true) => WrapperOutputKind::TaggedArray,
                        },
                        param,
                        inner_ty: to,
                    });
                }

                None
            })
            .collect()
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

#[derive(Debug, Default)]
struct WrapperCallArg {
    param: Option<WrapperParam>,
    value: TokenStream,
    generic_param: Option<TokenStream>,
}

impl WrapperCallArg {
    fn new(param: &CommandParam, wrapper_outputs: &[WrapperOutput]) -> WrapperCallArg {
        let name = param.decl.name;
        let ident = param_ident(name);

        if param.object_type_of.is_some() {
            return WrapperCallArg {
                value: quote! { T::TYPE },
                ..Default::default()
            };
        } else if param.object_handle_of.is_some() {
            return WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Tokens(quote! { T }),
                }),
                value: quote! { #ident.as_raw() },
                generic_param: Some(quote! { T: Handle }),
            };
        }

        let output = wrapper_outputs
            .iter()
            .find(|output| ptr::addr_eq(output.param, param));
        if let Some(output) = output {
            match output.kind {
                WrapperOutputKind::UntaggedSingle
                | WrapperOutputKind::UntaggedArrayWithInput { .. } => {
                    return WrapperCallArg {
                        value: quote! { #ident.as_mut_ptr() },
                        ..Default::default()
                    };
                }
                WrapperOutputKind::UntaggedArray => {
                    return WrapperCallArg {
                        value: if output.inner_ty.is_void() {
                            quote! { #ident.cast() }
                        } else {
                            quote! { #ident }
                        },
                        ..Default::default()
                    };
                }
                WrapperOutputKind::TaggedSingle => {
                    return WrapperCallArg {
                        param: Some(WrapperParam {
                            name,
                            ty: RustTy::Ref(Box::new(output.inner_ty.to_rust()), Mutability::Mut),
                        }),
                        value: quote! { #ident },
                        ..Default::default()
                    };
                }
                // todo
                WrapperOutputKind::TaggedArray => (),
            }
        }

        let length_of_output = wrapper_outputs.iter().find(|output| {
            matches!(
                output.param.length,
                Some(Length::DefinedByParam(len)) if param.decl.name == len
            )
        });
        if let Some(output) = length_of_output {
            let length_of = param_ident(output.param.decl.name);
            match output.kind {
                WrapperOutputKind::UntaggedSingle | WrapperOutputKind::TaggedSingle => {
                    unreachable!()
                }
                WrapperOutputKind::UntaggedArrayWithInput { .. } => {
                    return WrapperCallArg {
                        value: quote! { #length_of.len() as _ },
                        ..Default::default()
                    };
                }
                WrapperOutputKind::UntaggedArray => {
                    return WrapperCallArg {
                        value: quote! { #ident },
                        ..Default::default()
                    };
                }
                // todo
                WrapperOutputKind::TaggedArray => (),
            }
        } else if let Some(length_of) = param.length_of.first() {
            let length_of = param_ident(*length_of);
            return WrapperCallArg {
                value: quote! { #length_of.len() as _ },
                ..Default::default()
            };
        }

        match (&param.decl.ty, &param.length) {
            (
                &Ty::Ptr(Ty::CPrimary(CPrimaryType::Void), mutability),
                Some(Length::DefinedByParam(_)),
            ) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::ArrayRef(Box::new(RustTy::Tokens(quote! { u8 })), mutability, None),
                }),
                value: match mutability {
                    Mutability::Not => quote! { #ident.as_ptr().cast() },
                    Mutability::Mut => quote! { #ident.as_mut_ptr().cast() },
                },
                ..Default::default()
            },
            (
                Ty::Ptr(Ty::CPrimary(CPrimaryType::Char), Mutability::Not),
                Some(Length::NullTerminated),
            ) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::WithGenerics {
                        tokens: quote! { Option },
                        generic_args: vec![RustTy::Ref(
                            Box::new(RustTy::WithGenerics {
                                tokens: quote! { CStr },
                                generic_args: vec![],
                            }),
                            Mutability::Not,
                        )],
                    },
                }),
                value: quote! { #ident.map_or(ptr::null(), |s| s.as_ptr()) },
                ..Default::default()
            },
            (&Ty::ApiType(type_name), None) if type_name == TypeName::VK_BOOL32 => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Tokens(quote! { bool }),
                }),
                value: quote! { #ident as _ },
                ..Default::default()
            },
            (&Ty::ApiType(type_name), None)
                if type_name == TypeName::VK_INSTANCE || type_name == TypeName::VK_DEVICE =>
            {
                WrapperCallArg {
                    value: quote! { self.handle },
                    ..Default::default()
                }
            }
            (Ty::Ptr(to, Mutability::Not), None) if param.optional == [true] => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::WithGenerics {
                        tokens: quote! { Option },
                        generic_args: vec![RustTy::Ref(Box::new(to.to_rust()), Mutability::Not)],
                    },
                }),
                value: quote! {
                    match #ident {
                        Some(r) => r,
                        None => ptr::null_mut(),
                    }
                },
                ..Default::default()
            },
            (Ty::Ptr(to, Mutability::Not), None) if !to.is_opaque() => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::Ref(Box::new(to.to_rust()), Mutability::Not),
                }),
                value: quote! { #ident },
                ..Default::default()
            },
            (Ty::Ptr(to, Mutability::Not), Some(Length::DefinedByParam(_))) => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: RustTy::ArrayRef(Box::new(to.to_rust()), Mutability::Not, None),
                }),
                value: quote! { #ident.as_ptr() },
                ..Default::default()
            },
            _ => WrapperCallArg {
                param: Some(WrapperParam {
                    name,
                    ty: param.decl.ty.to_rust(),
                }),
                value: quote! { #ident },
                ..Default::default()
            },
        }
    }
}

fn wrapper(octx: &OutputContext, command: &Command, name: &Ident) -> Vec<Function> {
    let call_ret_result =
        matches!(command.return_type, Some(Ty::ApiType(name)) if name == TypeName::VK_RESULT);
    let wrapper_outputs = WrapperOutput::list(octx, command);

    let call_args: Vec<_> = command
        .params
        .values()
        .map(|param| WrapperCallArg::new(param, &wrapper_outputs))
        .collect();

    let mut generic_param_iter = call_args
        .iter()
        .filter_map(|call_arg| call_arg.generic_param.as_ref())
        .peekable();
    let has_generic_params = generic_param_iter.peek().is_some();
    let generic_params = has_generic_params.then(|| quote! { #(#generic_param_iter),* });

    let param_code = call_args.iter().filter_map(|call_arg| {
        call_arg.param.as_ref().map(|wrapper_param| {
            let ident = param_ident(wrapper_param.name);
            let ty = wrapper_param.ty.tokens(&**octx, None);
            quote! { #ident: #ty }
        })
    });
    let inputs = quote! { &self, #( #param_code ),* };

    let length_eq_asserts = command.params.values().flat_map(|param| {
        param.length_of.array_windows().map(|[a, b]| {
            let a = param_ident(*a);
            let b = param_ident(*b);
            quote! { assert_eq!(#a.len(), #b.len()); }
        })
    });

    let arg_code = call_args.iter().map(|param| &param.value);
    let call = quote! { (self.__table.#name)( #( #arg_code ),* ) };

    match wrapper_outputs.as_slice() {
        [] => {
            let return_value = if call_ret_result {
                quote! { #call.result() }
            } else {
                quote! { #call }
            };

            vec![Function {
                generic_params,
                inputs,
                output: if call_ret_result {
                    quote! { -> VkResult<()> }
                } else if let Some(return_ty) = &command.return_type {
                    let return_ty = return_ty.to_rust().tokens(&**octx, None);
                    quote! { -> #return_ty }
                } else {
                    quote! {}
                },
                content: quote! {
                    #(#length_eq_asserts)*
                    #return_value
                },
                ..Default::default()
            }]
        }
        [
            output @ WrapperOutput {
                kind: WrapperOutputKind::UntaggedSingle,
                ..
            },
        ] => {
            let param = param_ident(output.param.decl.name);
            let bool32 =
                matches!(output.inner_ty, &Ty::ApiType(name) if name == TypeName::VK_BOOL32);

            let return_value = match (call_ret_result, bool32) {
                (false, false) => quote! { #call; #param.assume_init() },
                (true, false) => quote! { #call.assume_init_on_success(#param) },
                (false, true) => quote! { #call; #param.assume_init() > 0 },
                (true, true) => quote! { #call.result()?; Ok(#param.assume_init() > 0) },
            };

            let inner_ty = output.inner_ty.to_rust().tokens(&**octx, None);
            let output = match (call_ret_result, bool32) {
                (false, false) => quote! { -> #inner_ty },
                (true, false) => quote! { -> VkResult<#inner_ty> },
                (false, true) => quote! { -> bool },
                (true, true) => quote! { -> VkResult<bool> },
            };

            vec![Function {
                generic_params,
                inputs,
                output,
                content: quote! {
                    #(#length_eq_asserts)*
                    let mut #param = MaybeUninit::uninit();
                    #return_value
                },
                ..Default::default()
            }]
        }
        [
            output @ WrapperOutput {
                kind: WrapperOutputKind::UntaggedArray,
                ..
            },
        ] => {
            let count = match &output.param.length {
                Some(Length::DefinedByParam(count)) => param_ident(*count),
                _ => unreachable!(),
            };

            let data = param_ident(output.param.decl.name);
            let explicit_data_ty = output.inner_ty.is_void().then(|| quote! { : *mut u8 });
            let inner_ret_ty = if output.inner_ty.is_void() {
                quote! { u8 }
            } else {
                output.inner_ty.to_rust().tokens(&**octx, None)
            };

            let closure_args = quote! { #count, #data #explicit_data_ty };
            let return_value = if call_ret_result {
                quote! { read_into_uninitialized_vector(|#closure_args| #call) }
            } else {
                quote! {
                    read_into_uninitialized_vector(|#closure_args| {
                        #call;
                        vk::Result::SUCCESS
                    })
                    .unwrap()
                }
            };
            vec![Function {
                generic_params,
                inputs,
                output: if call_ret_result {
                    quote! { -> VkResult<Vec<#inner_ret_ty>> }
                } else {
                    quote! { -> Vec<#inner_ret_ty> }
                },
                content: quote! {
                    #(#length_eq_asserts)*
                    #return_value
                },
                ..Default::default()
            }]
        }
        [
            output @ WrapperOutput {
                kind: WrapperOutputKind::UntaggedArrayWithInput { input },
                ..
            },
        ] => {
            let input = param_ident(*input);
            let param = param_ident(output.param.decl.name);
            let inner_ty = output.inner_ty.to_rust().tokens(&**octx, None);
            vec![Function {
                generic_params,
                inputs,
                output: quote! {
                    -> core::result::Result<Vec<#inner_ty>, (Vec<#inner_ty>, vk::Result)>
                },
                content: quote! {
                    let mut #param = Vec::with_capacity(#input.len());
                    let err_code = #call;
                    #param.set_len(#input.len());
                    match err_code {
                        vk::Result::SUCCESS => Ok(#param),
                        _ => Err((#param, err_code)),
                    }
                },
                ..Default::default()
            }]
        }
        [
            WrapperOutput {
                kind: WrapperOutputKind::TaggedSingle,
                ..
            },
        ] => {
            let return_value = if call_ret_result {
                quote! { #call.result() }
            } else {
                call
            };

            vec![Function {
                generic_params,
                inputs,
                output: if call_ret_result {
                    quote! { -> VkResult<()> }
                } else {
                    quote! {}
                },
                content: quote! {
                    #(#length_eq_asserts)*
                    #return_value
                },
                ..Default::default()
            }]
        }
        // todo: turn this into panic...
        _ => vec![],
    }
}
