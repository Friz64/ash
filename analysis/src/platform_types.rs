use crate::{
    decl::Ty,
    item::RequireMap,
    name::PlatformTypeName,
    xml::cdecl::{CDecl, CDeclMode},
};
use indexmap::IndexMap;
use std::sync::LazyLock;

pub const XML: &str = include_str!("platform-types.xml");

pub static PLATFORM_TYPES: LazyLock<IndexMap<PlatformTypeName, Ty>> = LazyLock::new(|| {
    let mut map = IndexMap::new();

    let doc = roxmltree::Document::parse(XML).unwrap();
    let root = doc.root_element();
    assert_eq!(root.tag_name().name(), "types");

    // platform types don't depend on types in the XML registry
    let empty_require_map = RequireMap::default();
    for type_node in root.children().filter(|node| node.has_tag_name("type")) {
        let c_decl = CDecl::from_xml(CDeclMode::TypeDef, type_node.children());
        let name = PlatformTypeName::new(c_decl.name);
        let ty = Ty::from_c(&empty_require_map, &c_decl.ty);
        map.insert(name, ty);
    }

    map
});
