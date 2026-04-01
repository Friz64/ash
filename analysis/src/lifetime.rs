use indexmap::IndexMap;
use proc_macro2::{Punct, Spacing, TokenStream};
use quote::{ToTokens, TokenStreamExt, format_ident};
use std::collections::HashMap;
use syn::Ident;

use crate::{
    decl::{Decl, Ty},
    item::{TypeItem, structure::StructMember},
    name::TypeName,
};

pub struct Lifetime(pub Ident);

impl Lifetime {
    pub fn placeholder() -> Self {
        Lifetime(format_ident!("_"))
    }
}

impl ToTokens for Lifetime {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Punct::new('\'', Spacing::Joint));
        self.0.to_tokens(tokens);
    }
}

pub fn lifetime_propagation(types: &IndexMap<TypeName, TypeItem>) -> HashMap<TypeName, bool> {
    let mut store = HashMap::new();
    for &type_name in types.keys() {
        determine_for_type(&mut store, types, &Ty::SpecType(type_name), false);
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
        Ty::SpecType(type_name) => {
            if let Some(existing_result) = store.get(type_name) {
                return *existing_result;
            }

            let result = match &types[type_name] {
                TypeItem::Alias(item) => {
                    determine_for_type(store, types, &Ty::SpecType(item.alias), false)
                }
                TypeItem::Struct(item) => item.members.iter().any(|member| {
                    matches!(
                        member,
                        StructMember::Normal(Decl {
                            ty,
                            ..
                        })if determine_for_type(store, types, ty, true)
                    )
                }),
                TypeItem::Union(item) => item.members.iter().any(|member| {
                    matches!(
                        member,
                        Decl {
                            ty,
                            ..
                        } if determine_for_type(store, types, ty, false)
                    )
                }),
                _ => false,
            };

            store.insert(*type_name, result);
            result
        }
        Ty::Ptr(..) if apply_for_pointers => true,
        Ty::Ptr(to, ..) => determine_for_type(store, types, to, false),
        Ty::Ref(..) => true,
        _ => false,
    }
}
