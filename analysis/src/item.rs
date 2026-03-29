pub mod alias;
pub mod basetype;
pub mod bitmask;
pub mod cmacro;
pub mod constant;
pub mod enumeration;
pub mod function;
pub mod handle;
pub mod structure;

use crate::{
    Library, LibraryName,
    item::{
        alias::Alias,
        basetype::BaseType,
        bitmask::BitMask,
        cmacro::CMacro,
        constant::Constant,
        enumeration::Enum,
        function::{Command, FuncPointer},
        handle::Handle,
        structure::{Struct, Union},
    },
    name::{CMacroName, CommandName, ConstantName, FuncPointerName, TypeName},
    xml::{self, Require, RequireCommand, RequireConstant, RequireType},
};
use indexmap::IndexMap;
use std::{
    collections::{HashMap, hash_map},
    iter,
};
use tinyvec::{ArrayVec, array_vec};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequiredBy {
    pub library: LibraryName,
    pub locations: ArrayVec<[RequireLocation; 4]>,
}

impl RequiredBy {
    pub fn primary_location(&self) -> RequireLocation {
        self.locations[0]
    }

    pub fn merge(&mut self, other: RequiredBy) {
        assert_eq!(self.library, other.library);
        for location in &other.locations {
            if !self.locations.contains(location) {
                self.locations.push(*location);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequireLocation {
    Core { major: u32, minor: u32 },
    Extension { name: &'static str },
}

impl Default for RequireLocation {
    fn default() -> Self {
        RequireLocation::Core { major: 1, minor: 0 }
    }
}

#[derive(Default)]
pub(crate) struct RequireMap {
    pub ty: HashMap<TypeName, RequiredBy>,
    pub func_pointer: HashMap<FuncPointerName, RequiredBy>,
    pub command: HashMap<CommandName, RequiredBy>,
    pub constant: HashMap<ConstantName, RequiredBy>,
    pub c_macro: HashMap<CMacroName, RequiredBy>,
}

pub trait Named<T> {
    fn name(&self) -> T;
}

#[derive(Debug)]
pub enum TypeItem {
    Alias(Alias),
    Struct(Struct),
    Union(Union),
    Enum(Enum),
    BitMask(BitMask),
    BitMaskBits { bitmask_name: TypeName },
    BaseType(BaseType),
    Handle(Handle),
}

#[derive(Default, Debug)]
pub struct Items {
    pub types: IndexMap<TypeName, TypeItem>,
    pub func_pointers: IndexMap<FuncPointerName, FuncPointer>,
    pub commands: IndexMap<CommandName, Command>,
    pub constants: IndexMap<ConstantName, Constant>,
    pub cmacros: IndexMap<CMacroName, CMacro>,
}

impl Items {
    pub fn collect(libraries: &[&Library]) -> Items {
        debug!("collecting items");
        let mut items = Items::default();

        let mut require_map = RequireMap::default();

        iter_requires(libraries, |library, location, require| {
            fn extend<T>(
                entry: hash_map::Entry<T, RequiredBy>,
                library: LibraryName,
                location: RequireLocation,
            ) {
                let required_by = entry.or_insert_with(|| RequiredBy {
                    library,
                    locations: ArrayVec::new(),
                });

                required_by.locations.push(location);
            }

            for require_type in &require.types {
                match require_type {
                    RequireType::Type(name) => {
                        extend(require_map.ty.entry(*name), library, location);
                    }
                    RequireType::CMacro(name) => {
                        extend(require_map.c_macro.entry(*name), library, location);
                    }
                    RequireType::FuncPointer(name) => {
                        extend(require_map.func_pointer.entry(*name), library, location);
                    }
                    // in ash these are covered by platform_types.rs, so let's just ignore those :P
                    RequireType::External(_name) => continue,
                };
            }

            for RequireConstant { name, .. } in &require.constants {
                extend(require_map.constant.entry(*name), library, location);
            }

            for RequireCommand { name } in &require.commands {
                extend(require_map.command.entry(*name), library, location);
            }
        });

        let bitmask_bits_map: HashMap<TypeName, &xml::BitMaskBits> = (libraries.iter())
            .flat_map(|library| (library.xml.bitmask_bits.iter()).map(|item| (item.name, item)))
            .collect();

        for library in libraries {
            items.collect_type(
                &library.xml.structs,
                |xml| Struct::new(&require_map, xml),
                TypeItem::Struct,
            );

            items.collect_type(
                &library.xml.struct_aliases,
                |xml| Alias::new(&require_map, xml),
                TypeItem::Alias,
            );

            items.collect_type(
                &library.xml.unions,
                |xml| Union::new(&require_map, xml),
                TypeItem::Union,
            );

            items.collect_type(
                &library.xml.enums,
                |xml| Enum::new(&require_map, xml),
                TypeItem::Enum,
            );

            items.collect_type(
                &library.xml.enum_aliases,
                |xml| Alias::new(&require_map, xml),
                TypeItem::Alias,
            );

            (items.types).extend(library.xml.bitmasks.iter().flat_map(|xml| {
                let mut bits_name_emitted = false;
                let mut bitmask = BitMask::new(&require_map, &bitmask_bits_map, xml);
                iter::from_fn(move || {
                    if !bits_name_emitted
                        && let Some(bitmask) = &bitmask
                        && let Some(bits_name) = bitmask.bits_name
                    {
                        bits_name_emitted = true;
                        let bitmask_name = bitmask.name();
                        return Some((bits_name, TypeItem::BitMaskBits { bitmask_name }));
                    }

                    bitmask.take().map(|ty| (ty.name(), TypeItem::BitMask(ty)))
                })
            }));

            items.collect_type(
                &library.xml.bitmask_aliases,
                |xml| Alias::new(&require_map, xml),
                TypeItem::Alias,
            );

            items.collect_type(
                &library.xml.basetypes,
                |xml| BaseType::new(&require_map, xml),
                TypeItem::BaseType,
            );

            items.collect_type(
                &library.xml.handles,
                |xml| Handle::new(&require_map, xml),
                TypeItem::Handle,
            );

            items.collect_type(
                &library.xml.handle_aliases,
                |xml| Alias::new(&require_map, xml),
                TypeItem::Alias,
            );

            Items::collect_item(
                &mut items.func_pointers,
                &library.xml.func_pointers,
                |xml| FuncPointer::new(&require_map, xml),
            );

            Items::collect_item(&mut items.constants, &library.xml.constants, |xml| {
                Constant::from_base_constant(&require_map, xml)
            });

            Items::collect_item(&mut items.cmacros, &library.xml.cmacros, |xml| {
                CMacro::new(&require_map, xml)
            });
        }

        iter_requires(libraries, |library, location, require| {
            let required_by = RequiredBy {
                library,
                locations: array_vec!([RequireLocation; _] => location),
            };

            for enumerator in &require.enumerators {
                match &mut items.types[&enumerator.extends] {
                    &mut TypeItem::BitMaskBits { bitmask_name } => {
                        let TypeItem::BitMask(bitmask) = &mut items.types[&bitmask_name] else {
                            unreachable!()
                        };

                        bitmask.extend(
                            required_by,
                            iter::once((enumerator.name, &enumerator.value)),
                        );
                    }
                    TypeItem::Enum(enumeration) => {
                        enumeration.extend(
                            required_by,
                            iter::once((enumerator.name, &enumerator.value)),
                        );
                    }
                    _ => unreachable!(),
                }
            }

            for constant in &require.constants {
                if let Some(constant) = Constant::from_require(required_by, constant) {
                    items.constants.insert(constant.name(), constant);
                }
            }

            for command in &require.commands {
                if let Some(command) = Command::from_require(required_by, command) {
                    items.commands.insert(command.name(), command);
                }
            }
        });

        items
    }

    fn collect_type<'a, X: 'a, T: Named<TypeName>>(
        &mut self,
        xml_src: impl IntoIterator<Item = &'a X>,
        construct: impl FnMut(&X) -> Option<T>,
        en: impl Fn(T) -> TypeItem,
    ) {
        self.types.extend(
            xml_src
                .into_iter()
                .filter_map(construct)
                .map(|ty| (ty.name(), en(ty))),
        )
    }

    fn collect_item<'a, X: 'a, N, T: Named<N>>(
        target: &mut impl Extend<(N, T)>,
        xml_src: impl IntoIterator<Item = &'a X>,
        construct: impl FnMut(&X) -> Option<T>,
    ) {
        target.extend(
            xml_src
                .into_iter()
                .filter_map(construct)
                .map(|ty| (ty.name(), ty)),
        );
    }
}

fn iter_requires(
    libraries: &[&Library],
    mut f: impl FnMut(LibraryName, RequireLocation, &Require),
) {
    for library in libraries {
        for feature in &library.xml.features {
            let location = RequireLocation::Core {
                major: feature.version.major,
                minor: feature.version.minor,
            };

            for require in &feature.requires {
                f(library.name(), location, require);
            }
        }

        for extension in &library.xml.extensions {
            let location = RequireLocation::Extension {
                name: extension.name,
            };

            for require in &extension.requires {
                f(library.name(), location, require);
            }
        }
    }
}
