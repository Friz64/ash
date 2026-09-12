use crate::{
    item::{NamedType, RequireMap, RequiredBy},
    name::{CommandName, TypeName},
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct TypeAlias {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub alias: TypeName,
}

impl NamedType for TypeAlias {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl TypeAlias {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::TypeAlias) -> Option<TypeAlias> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(TypeAlias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        })
    }
}

#[derive(Debug)]
pub struct CommandAlias {
    pub required_by: RequiredBy,
    pub name: CommandName,
    pub alias: CommandName,
}

impl CommandAlias {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::CommandAlias) -> Option<CommandAlias> {
        let required_by = *require_map.command.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(CommandAlias {
            required_by,
            name: xml.name,
            alias: xml.alias,
        })
    }
}
