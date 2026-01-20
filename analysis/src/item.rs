pub mod alias;
pub mod basetype;
pub mod bitmask;
pub mod constant;
pub mod enumeration;
pub mod funcpointer;
pub mod handle;
pub mod structure;

use crate::{
    decl,
    item::{
        alias::Alias,
        basetype::BaseType,
        bitmask::{BitMask, BitMaskBits},
        constant::Constant,
        enumeration::Enum,
        funcpointer::FuncPointer,
        handle::Handle,
        structure::{Struct, Union},
    },
    name::{ConstantName, TypeName},
    xml::Require,
    Library, LibraryName,
};
use indexmap::IndexMap;
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequiredBy {
    pub library: LibraryName,
    pub location: RequireLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequireLocation {
    Core { major: u32, minor: u32 },
    Extension { name: &'static str },
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
    pub constants: IndexMap<ConstantName, Constant>,
}

impl Items {
    pub fn collect(libraries: &[&Library]) -> Items {
        debug!("collecting items");
        let mut items = Items::default();

        let mut type_require_map = HashMap::new();
        let mut constant_require_map = HashMap::new();

        iter_requires(libraries, |required_by, require| {
            for require_type in &require.types {
                type_require_map.insert(require_type.name, required_by);
            }

            for require_command in &require.constants {
                constant_require_map.insert(require_command.name, required_by);
            }
        });

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

            // todo: das muss schöner gehen
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
                let name = TypeName(ty.name);
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                items.types.insert(
                    name,
                    TypeItem::FuncPointer(FuncPointer::new(&decl_ctx, required_by, ty)),
                );
            }

            for constant in &library.xml.constants {
                let name = constant.name;
                let Some(&required_by) = constant_require_map.get(&name) else {
                    continue;
                };

                items
                    .constants
                    .insert(name, Constant::from_constant(required_by, constant));
            }
        }

        iter_requires(libraries, |required_by, require| {
            for constant in &require.constants {
                let name = constant.name;
                if let Some(constant) = Constant::from_require_constant(required_by, constant) {
                    items.constants.insert(name, constant);
                }
            }
        });

        items
    }
}

fn iter_requires(libraries: &[&Library], mut f: impl FnMut(RequiredBy, &Require)) {
    for library in libraries {
        for feature in &library.xml.features {
            let required_by = RequiredBy {
                library: library.name,
                location: RequireLocation::Core {
                    major: feature.version.major,
                    minor: feature.version.minor,
                },
            };

            for require in &feature.requires {
                f(required_by, require);
            }
        }

        for extension in &library.xml.extensions {
            let required_by = RequiredBy {
                library: library.name,
                location: RequireLocation::Extension {
                    name: extension.name,
                },
            };

            for require in &extension.requires {
                f(required_by, require);
            }
        }
    }
}
