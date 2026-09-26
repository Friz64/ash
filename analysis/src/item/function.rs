use crate::{
    decl::{Decl, Ty},
    item::{RequireMap, RequiredBy},
    name::{CommandName, FuncPointerName, VariableName},
    xml::{
        self,
        cexpr::{self, CExprItems},
    },
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct FuncPointer {
    pub required_by: RequiredBy,
    pub name: FuncPointerName,
    pub params: Vec<Decl>,
    pub return_type: Option<Ty>,
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

#[derive(Debug, Clone)]
pub enum Length {
    DefinedByMember(VariableName),
    NullTerminated,
    Count(CExprItems),
}

#[derive(Debug)]
pub struct CommandParam {
    pub decl: Decl,
    pub length: Option<Length>,
    pub optional: Vec<bool>,
}

impl CommandParam {
    pub fn optional_at_depth(&self, depth: usize) -> Option<bool> {
        self.optional.get(depth).copied()
    }
}

#[derive(Debug)]
pub struct Command {
    pub required_by: RequiredBy,
    pub name: CommandName,
    pub params: Vec<CommandParam>,
    pub return_type: Option<Ty>,
}

impl Command {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Command) -> Option<Command> {
        let required_by = *require_map.command.get(&xml.name)?;
        trace!("constructing");

        let params = (xml.params.iter())
            .map(|param| {
                let len_slice = if param.altlen.is_empty() {
                    param.len.as_slice()
                } else {
                    param.altlen.as_slice()
                };

                let len_str = match len_slice {
                    [] => None,
                    [len] => Some(len),
                    _ => unimplemented!("expected there to be at most one length specifier"),
                };

                let length = len_str.map(|&len| {
                    if len == "null-terminated" {
                        Length::NullTerminated
                    } else if (xml.params.iter()).any(|xml_member| xml_member.c_decl.name == len) {
                        Length::DefinedByMember(VariableName::new(len))
                    } else {
                        Length::Count(cexpr::parse(len))
                    }
                });

                CommandParam {
                    decl: Decl::from_c(require_map, &param.c_decl),
                    length,
                    optional: param.optional.clone(),
                }
            })
            .collect();

        Some(Command {
            required_by,
            name: xml.name,
            params,
            return_type: (xml.return_type.as_ref()).map(|c_type| Ty::from_c(require_map, c_type)),
        })
    }
}
