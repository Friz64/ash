pub mod structure;

use self::structure::Structure;
use crate::{xml, Library};
use indexmap::IndexMap;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequiredBy {
    Feature { major: u32, minor: u32 },
    Extension { name: &'static str },
}

pub trait ItemInfo {
    fn required_by(&self) -> RequiredBy;

    fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum Item {
    Structure(Structure),
}

impl ItemInfo for Item {
    fn required_by(&self) -> RequiredBy {
        match self {
            Item::Structure(i) => i.required_by(),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Item::Structure(i) => i.name(),
        }
    }
}

impl Item {
    pub(crate) fn emerge(emerge_ctx: &mut EmergeCtx, name: &'static str) -> Option<&'static Item> {
        if let Some(value) = emerge_ctx.name_item_map.get(name) {
            return *value;
        }

        let v = (|| {
            let required_by = emerge_ctx.name_require_map.get(name)?;
            // really??
            let xml = emerge_ctx
                .registry
                .structs
                .iter()
                .find(|structure| structure.name == name)?;

            Some(&*Box::leak(Box::new(Item::Structure(Structure::new(
                emerge_ctx,
                *required_by,
                xml,
            )))))
        })();

        emerge_ctx.name_item_map.insert(name, v);
        *emerge_ctx.name_item_map.get(name).unwrap()
    }
}

pub(crate) struct EmergeCtx<'a> {
    registry: &'a xml::Registry,
    name_require_map: &'a HashMap<&'static str, RequiredBy>,
    name_item_map: HashMap<&'static str, Option<&'static Item>>,
}

#[derive(Default, Debug)]
pub struct Items {
    pub structures: Vec<&'static Structure>,
}

impl Items {
    pub(super) fn collect(&mut self, library: &Library) {
        let name_require_map = build_name_require_map(library);
        let mut emerge_ctx = EmergeCtx {
            registry: &library.xml,
            name_require_map: &name_require_map,
            name_item_map: HashMap::new(),
        };

        for structure in &library.xml.structs {
            let name = structure.name;
            let Some(&required_by) = name_require_map.get(name) else {
                continue;
            };

            let structure = Structure::new(&mut emerge_ctx, required_by, structure);
            self.structures.push(structure)
        }
    }
}

fn build_name_require_map(library: &Library) -> HashMap<&'static str, RequiredBy> {
    let mut name_require_map = HashMap::new();

    for feature in &library.xml.features {
        let required_by = RequiredBy::Feature {
            major: feature.version.major,
            minor: feature.version.minor,
        };

        for require in &feature.requires {
            for require_type in &require.types {
                name_require_map.insert(require_type.name, required_by);
            }
        }
    }

    for extension in &library.xml.extensions {
        let required_by = RequiredBy::Extension {
            name: extension.name,
        };

        for require in &extension.requires {
            for require_type in &require.types {
                name_require_map.insert(require_type.name, required_by);
            }
        }
    }

    name_require_map
}
