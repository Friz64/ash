use crate::{
    item::{RequireMap, RequiredBy},
    name::CMacroName,
    xml::{self, cexpr::CExprItems},
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct CMacro {
    pub required_by: RequiredBy,
    pub name: CMacroName,
    pub args: Vec<&'static str>,
    pub cexpr: CExprItems,
}

impl CMacro {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::CMacro) -> Option<CMacro> {
        let required_by = *require_map.c_macro.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(CMacro {
            required_by,
            name: xml.name,
            args: xml.args.clone(),
            cexpr: xml.cexpr.clone(),
        })
    }

    pub fn has_args(&self) -> bool {
        self.args.is_empty()
    }
}
