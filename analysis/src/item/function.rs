use crate::{
    item::{Named, RequireMap, RequiredBy},
    xml::{
        self,
        name::{CommandName, FuncPointerName},
    },
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct FuncPointer {
    pub required_by: RequiredBy,
    pub name: FuncPointerName,
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
        })
    }
}

#[derive(Debug)]
pub struct Command {
    pub required_by: RequiredBy,
    pub name: CommandName,
}

impl Named<CommandName> for Command {
    fn name(&self) -> CommandName {
        self.name
    }
}

impl Command {
    #[instrument]
    pub(crate) fn from_require(
        required_by: RequiredBy,
        xml: &xml::RequireCommand,
    ) -> Option<Command> {
        trace!("constructing");

        Some(Command {
            required_by,
            name: xml.name,
        })
    }
}
