use crate::{
    decl::{Decl, Ty},
    item::{NamedType, RequireMap, RequiredBy},
    name::{EnumeratorName, TypeName, VariableName},
    xml,
};
use std::{cmp::Ordering, ops::Range};
use tracing::{instrument, trace};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Length {
    DefinedByMember(VariableName),
    NullTerminated,
    Count(usize),
    Custom(&'static str),
}

#[derive(Debug)]
pub struct RegularMember {
    pub decl: Decl,
    pub lengths: Vec<Length>,
}

impl RegularMember {
    /// For example, if the type is `const char* const*` (a pointer to an array of string pointers),
    /// then this returns [`Length::Count`] with the array length at depth 0,
    /// and [`Length::NullTerminated`] at depth 1.
    pub fn length_at_depth(&self, depth: usize) -> Option<Length> {
        self.lengths.get(depth).copied()
    }
}

#[derive(Debug)]
pub struct BitfieldMemberRange {
    pub decl: Decl,
    pub range: Range<u8>,
}

#[derive(Debug)]
pub enum Member {
    Regular(RegularMember),
    /// In every current case, this member is 32 bits in size.
    Bitfield(Vec<BitfieldMemberRange>),
}

#[derive(Debug)]
pub struct Struct {
    pub required_by: RequiredBy,
    pub name: TypeName,
    pub extends: Vec<TypeName>,
    pub structure_type: Option<EnumeratorName>,
    pub members: Vec<Member>,
}

impl NamedType for Struct {
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
            let len_slice = if member.altlen.is_empty() {
                member.len.as_slice()
            } else {
                member.altlen.as_slice()
            };

            let lengths = len_slice
                .iter()
                .map(|&len| {
                    if len == "null-terminated" {
                        Length::NullTerminated
                    } else if (xml.members.iter()).any(|xml_member| xml_member.c_decl.name == len) {
                        Length::DefinedByMember(VariableName::new(len))
                    } else if let Ok(count) = len.parse() {
                        Length::Count(count)
                    } else {
                        Length::Custom(len)
                    }
                })
                .collect();

            let decl = Decl::from_c(require_map, &member.c_decl);
            if let Some(width) = member.c_decl.bitfield_width {
                // this is currently the case everywhere,
                // and if this assumption is broken, the code below will panic
                const TY_WIDTH: u8 = 32;

                if used_bitwidth.is_none() {
                    used_bitwidth = Some(0);
                    members.push(Member::Bitfield(vec![]));
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
                    let Some(Member::Bitfield(ranges)) = members.last_mut() else {
                        unreachable!()
                    };

                    ranges.push(BitfieldMemberRange { decl, range });
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
                members.push(Member::Regular(RegularMember { decl, lengths }));
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

impl NamedType for Union {
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
