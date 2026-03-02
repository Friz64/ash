use crate::{
    item::{Named, RequireMap, RequiredBy},
    xml::{self, cexpr::CExprItems, name::CMacroName},
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct CMacro {
    pub required_by: RequiredBy,
    pub name: CMacroName,
    pub args: Vec<&'static str>,
    pub cexpr: CExprItems,
}

impl Named<CMacroName> for CMacro {
    fn name(&self) -> CMacroName {
        self.name
    }
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
}
