use crate::{
    decl::{Decl, Ty},
    item::{Named, RequireMap, RequiredBy},
    name::{EnumeratorName, TypeName, VariableName},
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
    Normal(StructDecl),
    BitField(Vec<BitfieldRange>),
}

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub extends: Vec<TypeName>,
    pub structure_type: Option<EnumeratorName>,
    pub members: Vec<StructMember>,
}

#[derive(Debug)]
pub struct StructDecl {
    pub decl: Decl,
    pub len: Vec<Length>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Length {
    Member(VariableName),
    NullTerminated,
    Pointer,
    Custom(&'static str),
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

        let extends = xml
            .structextends
            .iter()
            .map(|&extending| TypeName::new(extending))
            .collect();

        let mut structure_type = None;

        for member in &xml.members {
            let lens = if member.len.iter().any(|&s| s.starts_with("latexmath:")) {
                &member.altlen
            } else {
                &member.len
            };

            let len = lens
                .iter()
                .map(|&s| match s {
                    "null-terminated" => Length::NullTerminated,
                    "1" => Length::Pointer,
                    name if xml.members.iter().any(|mem| mem.c_decl.name == name) => {
                        Length::Member(VariableName::new(name))
                    }
                    custom => Length::Custom(custom),
                })
                .collect::<Vec<_>>();
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
                // should exist only once
                if let Some(value) = member.values
                    && let Ty::ApiType(ty) = decl.ty
                    && ty == TypeName::VK_STRUCTURE_TYPE
                    && decl.name.original() == "sType"
                {
                    structure_type = Some(EnumeratorName::new(value));
                }
                members.push(StructMember::Normal(StructDecl { decl, len }));
            }
        }

        Some(Struct {
            required_by,
            name: xml.name,
            extends,
            structure_type,
            members,
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
