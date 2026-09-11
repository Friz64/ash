use crate::{
    decl::{Decl, Ty},
    item::{Named, RequireMap, RequiredBy},
    name::{CommandName, FuncPointerName},
    xml::{self},
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct FuncPointer {
    pub required_by: RequiredBy,
    pub name: FuncPointerName,
    pub params: Vec<Decl>,
    pub return_type: Option<Ty>,
}

impl Named<FuncPointerName> for FuncPointer {
    fn name(&self) -> FuncPointerName {
        self.name
    }
}

impl FuncPointer {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::FuncPointer) -> Option<FuncPointer> {
        let required_by = *require_map.func_pointer.get(&xml.name)?;
        trace!("constructing");

        Some(FuncPointer {
            required_by,
            name: xml.name,
            params: (xml.params.iter())
                .map(|c_decl| Decl::from_c(require_map, c_decl))
                .collect(),
            return_type: (xml.return_type.as_ref()).map(|c_type| Ty::from_c(require_map, c_type)),
        })
    }
}

#[derive(Debug)]
pub struct CommandParam {
    pub decl: Decl,
}

#[derive(Debug)]
pub struct Command {
    pub required_by: RequiredBy,
    pub name: CommandName,
    pub params: Vec<CommandParam>,
    pub return_type: Option<Ty>,
}

impl Named<CommandName> for Command {
    fn name(&self) -> CommandName {
        self.name
    }
}

impl Command {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Command) -> Option<Command> {
        let required_by = *require_map.command.get(&xml.name)?;
        trace!("constructing");

        Some(Command {
            required_by,
            name: xml.name,
            params: (xml.params.iter())
                .map(|param| CommandParam {
                    decl: Decl::from_c(require_map, &param.c_decl),
                })
                .collect(),
            return_type: (xml.return_type.as_ref()).map(|c_type| Ty::from_c(require_map, c_type)),
        })
    }
}
