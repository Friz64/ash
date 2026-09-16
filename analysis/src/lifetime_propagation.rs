use indexmap::IndexMap;
use std::collections::HashMap;

use crate::{
    decl::{Decl, Ty},
    item::{
        TypeItem,
        structure::{Member, RegularMember},
    },
    name::TypeName,
};

pub fn run(types: &IndexMap<TypeName, TypeItem>) -> HashMap<TypeName, bool> {
    let mut store = HashMap::new();
    for &type_name in types.keys() {
        determine_for_type(&mut store, types, &Ty::ApiType(type_name), false);
    }

    store
}

fn determine_for_type(
    store: &mut HashMap<TypeName, bool>,
    types: &IndexMap<TypeName, TypeItem>,
    ty: &Ty,
    apply_for_pointers: bool,
) -> bool {
    match ty {
        Ty::ApiType(type_name) => {
            if let Some(existing_result) = store.get(type_name) {
                return *existing_result;
            }

            let result = match &types[type_name] {
                TypeItem::Alias(item) => {
                    determine_for_type(store, types, &Ty::ApiType(item.alias), false)
                }
                TypeItem::Struct(item) => item.members.iter().any(|member| {
                    matches!(
                        member,
                        Member::Regular(RegularMember { decl: Decl { ty, .. }, ..})
                            if determine_for_type(store, types, ty, true)
                    )
                }),
                TypeItem::Union(item) => item.members.iter().any(|member| {
                    matches!(
                        member,
                        Decl { ty, .. }
                            if determine_for_type(store, types, ty, false)
                    )
                }),
                _ => false,
            };

            store.insert(*type_name, result);
            result
        }
        Ty::Ptr(..) if apply_for_pointers => true,
        Ty::Ptr(to, ..) => determine_for_type(store, types, to, false),
        _ => false,
    }
}
