use crate::{
    decl::{Decl, Ty},
    item::{Named, RequireMap, RequiredBy},
    name::TypeName,
    xml,
};
use std::{cmp::Ordering, ops::Range};
use tracing::{instrument, trace};

#[derive(Debug)]
pub struct BitfieldRange {
    pub decl: Decl,
    pub range: Range<u8>,
}

#[derive(Debug)]
pub enum StructMember {
    Normal(Decl),
    BitField(Vec<BitfieldRange>),
}

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub members: Vec<StructMember>,
}

impl Named<TypeName> for Struct {
    fn name(&self) -> TypeName {
        self.name
    }
}

impl Struct {
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Structure) -> Option<Struct> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        let mut members = Vec::new();
        let mut used_bitwidth = None;
        for member in &xml.members {
            let decl = Decl::from_c(require_map, &member.c_decl);
            if let Some(width) = member.c_decl.bitfield_width {
                // this is currently the case everywhere,
                // and if this assumption is broken, the code below will panic
                const TY_WIDTH: u8 = 32;

                if used_bitwidth.is_none() {
                    used_bitwidth = Some(0);
                    members.push(StructMember::BitField(vec![]));
                }

                let currently_used_bitwidth = used_bitwidth.as_mut().unwrap();
                let last_used_bitwidth = *currently_used_bitwidth;
                *currently_used_bitwidth += width.get();
                let range = last_used_bitwidth..(*currently_used_bitwidth);

                match (*currently_used_bitwidth).cmp(&TY_WIDTH) {
                    Ordering::Less => (),
                    Ordering::Equal => used_bitwidth = None,
                    Ordering::Greater => panic!("bitfield should be large enough"),
                }

                if decl.name.original() != "reserved" {
                    let Some(StructMember::BitField(ranges)) = members.last_mut() else {
                        unreachable!()
                    };

                    ranges.push(BitfieldRange { decl, range });
                }
            } else {
                assert_eq!(used_bitwidth, None, "bitfield not fully used");
                members.push(StructMember::Normal(decl));
            }
        }

        Some(Struct {
            required_by,
            name: xml.name,
            members,
        })
    }

    pub fn has_pointer(&self) -> bool {
        self.members.iter().any(|member| {
            matches!(
                member,
                StructMember::Normal(Decl {
                    ty: Ty::Ptr(..),
                    ..
                })
            )
        })
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
    #[instrument(skip(require_map))]
    pub(crate) fn new(require_map: &RequireMap, xml: &xml::Structure) -> Option<Union> {
        let required_by = *require_map.ty.get(&xml.name)?;
        trace!(?required_by, "constructing");

        Some(Union {
            required_by,
            name: xml.name,
            members: (xml.members.iter())
                .map(|member| Decl::from_c(require_map, &member.c_decl))
                .collect(),
        })
    }

    pub fn has_pointer(&self) -> bool {
        self.members
            .iter()
            .any(|member| matches!(member.ty, Ty::Ptr(..)))
    }
}
