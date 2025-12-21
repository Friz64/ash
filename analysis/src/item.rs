pub mod alias;
pub mod basetype;
pub mod bitmask;
pub mod enumeration;
pub mod function;
pub mod handle;
pub mod structure;

use crate::{
    decl,
    item::{
        alias::Alias,
        basetype::BaseType,
        bitmask::{BitMask, BitMaskBits},
        enumeration::Enum,
        function::FuncPointer,
        handle::Handle,
        structure::{Struct, Union},
    },
    name::TypeName,
    Library,
};
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequiredBy {
    Feature { major: u32, minor: u32 },
    Extension { name: &'static str },
}

pub trait ItemInfo {
    fn required_by(&self) -> RequiredBy;

    fn name(&self) -> TypeName;
}

#[derive(Debug)]
pub enum TypeItem {
    Alias(Alias),
    Struct(Struct),
    Union(Union),
    Enum(Enum),
    BitMask(BitMask),
    BitMaskBits(BitMaskBits),
    BaseType(BaseType),
    Handle(Handle),
    FuncPointer(FuncPointer),
}

#[derive(Default, Debug)]
pub struct Items {
    pub types: IndexMap<TypeName, TypeItem>,
}

impl Items {
    pub(super) fn collect(libraries: &[&Library]) -> Items {
        let mut items = Items::default();

        let type_require_map = build_type_require_map(libraries);
        let decl_ctx = decl::Context::new(&type_require_map);

        for library in libraries {
            macro_rules! collect_type {
                ($iter_field:ident => $type_item_fn:expr) => {
                    for ty in &library.xml.$iter_field {
                        let name = ty.name;
                        let Some(&required_by) = type_require_map.get(&name) else {
                            continue;
                        };

                        items.types.insert(name, ($type_item_fn)(required_by, ty));
                    }
                };
            }

            collect_type!(structs =>
                |required_by, ty| TypeItem::Struct(Struct::new(&decl_ctx, required_by, ty)));
            collect_type!(struct_aliases =>
                |required_by, ty| TypeItem::Alias(Alias::new(required_by, ty)));
            collect_type!(unions =>
                |required_by, ty| TypeItem::Union(Union::new(&decl_ctx, required_by, ty)));
            collect_type!(enums =>
                |required_by, ty| TypeItem::Enum(Enum::new(required_by, ty)));
            collect_type!(enum_aliases =>
                |required_by, ty| TypeItem::Alias(Alias::new(required_by, ty)));
            collect_type!(bitmasks =>
                |required_by, ty| TypeItem::BitMask(BitMask::new(required_by, ty)));
            collect_type!(bitmask_aliases =>
                |required_by, ty| TypeItem::Alias(Alias::new(required_by, ty)));
            collect_type!(bitmask_bits =>
                |required_by, ty| TypeItem::BitMaskBits(BitMaskBits::new(required_by, ty)));
            collect_type!(basetypes =>
                |required_by, ty| TypeItem::BaseType(BaseType::new(required_by, ty)));
            collect_type!(handles =>
                |required_by, ty| TypeItem::Handle(Handle::new(required_by, ty)));
            collect_type!(handle_aliases =>
                |required_by, ty| TypeItem::Alias(Alias::new(required_by, ty)));

            for ty in &library.xml.funcpointers {
                let name = TypeName(ty.c_decl.name);
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                items.types.insert(
                    name,
                    TypeItem::FuncPointer(FuncPointer::new(required_by, ty)),
                );
            }
        }

        items
    }
}

fn build_type_require_map(libraries: &[&Library]) -> HashMap<TypeName, RequiredBy> {
    let mut type_require_map = HashMap::new();

    for library in libraries {
        for feature in &library.xml.features {
            let required_by = RequiredBy::Feature {
                major: feature.version.major,
                minor: feature.version.minor,
            };

            for require in &feature.requires {
                for require_type in &require.types {
                    type_require_map.insert(require_type.name, required_by);
                }
            }
        }

        for extension in &library.xml.extensions {
            let required_by = RequiredBy::Extension {
                name: extension.name,
            };

            for require in &extension.requires {
                for require_type in &require.types {
                    type_require_map.insert(require_type.name, required_by);
                }
            }
        }
    }

    type_require_map
}
