pub mod basetype;
pub mod enumeration;
pub mod handle;
pub mod structure;

use self::structure::Structure;
use crate::{
    decl,
    item::{basetype::BaseType, enumeration::Enumeration, handle::Handle},
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
    Structure(Structure),
    Enumeration(Enumeration),
    BaseType(BaseType),
    Handle(Handle),
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
            for structure in &library.xml.structs {
                let name = structure.name;
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                let structure = Structure::new(&decl_ctx, required_by, structure);
                items.types.insert(name, TypeItem::Structure(structure));
            }

            for enumeration in &library.xml.enums {
                let name = enumeration.name;
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                let enumeration = Enumeration::new(required_by, enumeration);
                items.types.insert(name, TypeItem::Enumeration(enumeration));
            }

            for basetype in &library.xml.basetypes {
                let name = basetype.name;
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                let basetype = BaseType::new(required_by, basetype);
                items.types.insert(name, TypeItem::BaseType(basetype));
            }

            for handle in &library.xml.handles {
                let name = handle.name;
                let Some(&required_by) = type_require_map.get(&name) else {
                    continue;
                };

                let handle = Handle::new(required_by, handle);
                items.types.insert(name, TypeItem::Handle(handle));
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
