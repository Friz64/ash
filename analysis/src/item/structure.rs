use crate::{
    decl::{Decl, Ty},
    item::{Named, RequiredBy, TypeRequireMap},
    name::TypeName,
    xml,
};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl Named<TypeName> for Struct {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Struct {
    #[instrument(skip(trm))]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::Structure) -> Option<Struct> {
        let required_by = *trm.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Struct {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(trm, &member.c_decl))
                .collect(),
        })
    }

    pub fn has_pointer(&self) -> bool {
        has_pointer(&self.members)
    }
}

#[derive(Debug)]
pub struct Union {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<Decl>,
}

impl Named<TypeName> for Union {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Union {
    #[instrument(skip(trm))]
    pub(crate) fn new(trm: &TypeRequireMap, xml: &xml::Structure) -> Option<Union> {
        let required_by = *trm.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Union {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(trm, &member.c_decl))
                .collect(),
        })
    }

    pub fn has_pointer(&self) -> bool {
        has_pointer(&self.members)
    }
}

fn has_pointer(members: &[Decl]) -> bool {
    members
        .iter()
        .any(|member| matches!(member.ty, Ty::Ptr(..)))
}
