use crate::{
    decl::Decl,
    item::{EmergeCtx, ItemInfo, RequiredBy},
    xml,
};

#[derive(Debug)]
pub struct Structure {
    pub required_by: RequiredBy,
    pub name: &'static str,
    pub members: Vec<Decl>,
}

impl ItemInfo for Structure {
    fn required_by(&self) -> RequiredBy {
        self.required_by
    }

    fn name(&self) -> &'static str {
        self.name
    }
}

impl Structure {
    pub(crate) fn new(
        emerge_ctx: &mut EmergeCtx,
        required_by: RequiredBy,
        xml: &xml::Structure,
    ) -> Structure {
        Structure {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(emerge_ctx, &member.c_decl))
                .collect(),
        }
    }
}
