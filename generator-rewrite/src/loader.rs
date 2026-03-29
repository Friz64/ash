use crate::{
    Context,
    output::{CodeMap, Destination},
};
use analysis::{
    decl::Ty,
    item::{CommandItem, RequireLocation, RequiredBy, function::Command},
    name::TypeName,
    to_rust::RustTranslator,
};
use heck::ToSnekCase;
use quote::{format_ident, quote};
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

    fn table_name(self, required_by: RequiredBy) -> Ident {
        match (self, required_by.primary_location()) {
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

    let mut tables: CodeMap<(FunctionType, RequiredBy)> = Default::default();
    for command_item in ctx.items.commands.values() {
        let (name, required_by, command) = match command_item {
            CommandItem::Alias(alias) => match &ctx.items.commands[&alias.alias] {
                CommandItem::Alias(..) => unreachable!(),
                CommandItem::Command(command) => (alias.name, alias.required_by, command),
            },
            CommandItem::Command(command) => (command.name, command.required_by, command),
        };

        let field_name = format_ident!("{}", name.original().to_snek_case());
        let command_ty = ctx.command_to_rust(name, true);

        let code = quote! {
            pub #field_name: #command_ty,
        };

        let function_type = FunctionType::of_command(command);
        tables.extend(CodeMap::new((function_type, required_by), code));
    }

    for (&(function_type, required_by), code) in tables.iter() {
        let table_name = function_type.table_name(required_by);

        let code = quote! {
            pub struct #table_name {
                #code
            }
        };

        codemap.extend(CodeMap::new(Destination::library(required_by), code));
    }
}
