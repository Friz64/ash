use crate::{
    decl::{Decl, Ty},
    item::{RequireMap, RequiredBy},
    name::{CommandName, FuncPointerName, VariableName},
    xml::{
        self,
        cexpr::{self, CExprItems},
    },
};
use indexmap::IndexMap;
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
    DefinedByParam(VariableName),
    NullTerminated,
    Count(CExprItems),
}

#[derive(Debug)]
pub struct CommandParam {
    pub decl: Decl,
    pub length: Option<Length>,
    /// All other params whose length is defined by this param
    pub length_of: Vec<VariableName>,
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
    pub params: IndexMap<VariableName, CommandParam>,
    pub return_type: Option<Ty>,
}

impl Command {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Command) -> Option<Command> {
        let required_by = *require_map.command.get(&xml.name)?;
        trace!("constructing");

        let mut params: Vec<CommandParam> = (xml.params.iter())
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
                        Length::DefinedByParam(VariableName::new(len))
                    } else {
                        Length::Count(cexpr::parse(len))
                    }
                });

                CommandParam {
                    decl: Decl::from_c(require_map, &param.c_decl),
                    length,
                    length_of: Vec::new(), // filled in second pass
                    optional: param.optional.clone(),
                }
            })
            .collect();

        // for each param `a`, check every other param `b`,
        // and find out if `a` determines the length of `b`
        for a_idx in 0..params.len() {
            for b_idx in 0..params.len() {
                let Ok([a, b]) = params.get_disjoint_mut([a_idx, b_idx]) else {
                    continue;
                };

                if matches!(b.length, Some(Length::DefinedByParam(name)) if name == a.decl.name) {
                    a.length_of.push(b.decl.name);
                }
            }
        }

        Some(Command {
            required_by,
            name: xml.name,
            params: params
                .into_iter()
                .map(|param| (param.decl.name, param))
                .collect(),
            return_type: (xml.return_type.as_ref()).map(|c_type| Ty::from_c(require_map, c_type)),
        })
    }
}
