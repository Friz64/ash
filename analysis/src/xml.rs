pub mod cdecl;
pub mod cexpr;
pub mod depends;
pub mod name;

use crate::LibraryName;
use cdecl::{CDecl, CDeclMode, CTok, CType};
use cexpr::{CExprItem, CExprItems};
use depends::Depends;
use name::{CMacroName, CommandName, ConstantName, FuncPointerName, TypeName};
use roxmltree::NodeType;
use roxmltree::StringStorage;
use std::fmt::Write;
use tracing::{info_span, trace};

/// A node with its `'input` lifetime set to `'static`.
type Node<'a> = roxmltree::Node<'a, 'static>;

/// Converts `roxmltree`'s `StringStorage` to a `&'static str`.
///
/// In nearly all cases this function will give you a slice from the original XML input,
/// but this is not always possible, for example when `&quot;` gets replaced with normal quotes.
/// This does not happen often, so we are leaking that memory for convenience.
fn leak(string_storage: StringStorage<'static>) -> &'static str {
    match string_storage {
        StringStorage::Borrowed(s) => s,
        StringStorage::Owned(s) => String::leak((*s).into()),
    }
}

/// Retrieves the value of the `node`'s attribute named `name`.
fn attribute(node: Node, name: &str) -> Option<&'static str> {
    node.attribute_node(name)
        .map(|attr| leak(attr.value_storage().clone()))
}

/// Retrieves the ','-separated values of the `node`'s attribute named `name`.
fn attribute_comma_separated(node: Node, name: &str) -> Vec<&'static str> {
    attribute(node, name)
        .map(|value| value.split(',').collect())
        .unwrap_or_default()
}

/// Retrieves the text inside the `node`
fn text(node: Node) -> &'static str {
    leak(node.text_storage().unwrap().clone())
}

/// Retrieves the text inside the next child element of `node` named `name`.
fn child_text(node: Node, name: &str) -> Option<&'static str> {
    node.children()
        .find(|node| node.has_tag_name(name))
        .map(text)
}

/// Returns [`true`] when the `node`'s "api" attribute matches the `expected` API.
fn api_matches(node: &Node, expected: &str) -> bool {
    node.attribute("api")
        .map(|values| values.split(',').any(|value| value == expected))
        .unwrap_or(true)
}

/// Returns a "pseudo-XML" representation of the node, for use in tracing spans.
fn node_span_field(node: &Node) -> String {
    let mut output = format!("<{:?}", node.tag_name());
    for attr in node.attributes() {
        write!(output, " {}='{}'", attr.name(), attr.value()).unwrap();
    }

    output + ">"
}

impl CDecl<'static> {
    fn from_xml(mode: CDeclMode, children: roxmltree::Children<'_, 'static>) -> CDecl<'static> {
        let mut c_tokens = vec![];
        for child in children {
            match child.node_type() {
                NodeType::Text => {
                    CTok::lex_into(text(child), &mut c_tokens).unwrap();
                }
                NodeType::Element => {
                    assert_eq!(child.attributes().len(), 0);
                    let text = || {
                        assert_eq!(child.children().count(), 1);
                        text(child)
                    };
                    c_tokens.push(match child.tag_name().name() {
                        "comment" => continue,
                        "type" => CTok::TypeName(text()),
                        "enum" => CTok::ValueName(text()),
                        "name" => CTok::DeclName(text()),
                        tag => unreachable!("unexpected `<{tag}>` in C declaration"),
                    })
                }
                NodeType::Root | NodeType::PI | NodeType::Comment => unreachable!(),
            }
        }

        c_tokens.retain_mut(|tok| {
            if let CTok::StrayIdent(name) = tok {
                match &name[..] {
                    // HACK(eddyb) work around `video.xml` spec bug (missing `<enum>`).
                    "STD_VIDEO_H264_MAX_NUM_LIST_REF" | "STD_VIDEO_H265_MAX_NUM_LIST_REF" => {
                        *tok = CTok::ValueName(name);
                    }

                    // HACK(eddyb) work around `vk.xml` spec bug (missing `<type>`).
                    "VkBool32" | "PFN_vkVoidFunction" => {
                        *tok = CTok::TypeName(name);
                    }

                    _ => {}
                }
            }

            match tok {
                // HACK(eddyb) ideally we'd expand this to something using the
                // C++11/C23 `[[...]]` attribute syntax, but that'd need support
                // in `cdecl`, and it's redundant since all function pointers
                // equally get it, so we can just remove it here.
                CTok::StrayIdent("VKAPI_PTR") => false,

                _ => true,
            }
        });

        CDecl::parse(mode, &c_tokens).unwrap()
    }
}

/// Raw representation of Vulkan XML files (`vk.xml`, `video.xml`).
#[derive(Debug, Default)]
pub struct Registry {
    pub externals: Vec<External>,
    pub basetypes: Vec<BaseType>,
    pub bitmasks: Vec<BitMask>,
    pub bitmask_aliases: Vec<TypeAlias>,
    pub handles: Vec<Handle>,
    pub handle_aliases: Vec<TypeAlias>,
    pub enum_types: Vec<EnumType>,
    pub enum_aliases: Vec<TypeAlias>,
    pub func_pointers: Vec<FuncPointer>,
    pub structs: Vec<Structure>,
    pub struct_aliases: Vec<TypeAlias>,
    pub unions: Vec<Structure>,
    pub cmacros: Vec<CMacro>,
    pub constants: Vec<BaseConstant>,
    pub enums: Vec<Enum>,
    pub bitmask_bits: Vec<BitMaskBits>,
    pub commands: Vec<Command>,
    pub command_aliases: Vec<CommandAlias>,
    pub features: Vec<Feature>,
    pub extensions: Vec<Extension>,
}

impl Registry {
    pub fn parse(input: &'static str, library_name: LibraryName, api: &str) -> Registry {
        let doc = roxmltree::Document::parse(input).unwrap();
        Registry::from_node(doc.root_element(), library_name, api)
    }

    fn from_node(registry_node: Node, library_name: LibraryName, api: &str) -> Registry {
        let mut registry = Registry::default();
        for registry_child in registry_node
            .children()
            .filter(|node| api_matches(node, api))
        {
            match registry_child.tag_name().name() {
                "types" => {
                    for type_node in registry_child
                        .children()
                        .filter(|node| node.has_tag_name("type"))
                        .filter(|node| api_matches(node, api))
                    {
                        let _s = info_span!("type", node = node_span_field(&type_node)).entered();
                        trace!("encountered node");
                        if type_node.has_attribute("alias") {
                            match type_node.attribute("category") {
                                Some("bitmask") => {
                                    registry
                                        .bitmask_aliases
                                        .push(TypeAlias::from_node(type_node));
                                }
                                Some("handle") => {
                                    registry
                                        .handle_aliases
                                        .push(TypeAlias::from_node(type_node));
                                }
                                Some("enum") => {
                                    registry.enum_aliases.push(TypeAlias::from_node(type_node));
                                }
                                Some("struct") => {
                                    registry
                                        .struct_aliases
                                        .push(TypeAlias::from_node(type_node));
                                }
                                _ => trace!("ignored"),
                            }
                        } else {
                            match type_node.attribute("category") {
                                Some("basetype") => {
                                    if let Some(basetype) = BaseType::from_node(type_node) {
                                        registry.basetypes.push(basetype);
                                    }
                                }
                                Some("bitmask") => {
                                    registry.bitmasks.push(BitMask::from_node(type_node))
                                }
                                Some("handle") => {
                                    registry.handles.push(Handle::from_node(type_node))
                                }
                                Some("enum") => {
                                    registry.enum_types.push(EnumType::from_node(type_node))
                                }
                                Some("funcpointer") => registry
                                    .func_pointers
                                    .push(FuncPointer::from_node(type_node, api)),
                                Some("struct") => {
                                    registry.structs.push(Structure::from_node(type_node, api))
                                }
                                Some("union") => {
                                    registry.unions.push(Structure::from_node(type_node, api));
                                }
                                Some("define") => {
                                    if let Some(define) = CMacro::from_node(type_node) {
                                        registry.cmacros.push(define);
                                    }
                                }
                                Some(_) => trace!("ignored"),
                                None => {
                                    registry.externals.push(External::from_node(type_node));
                                }
                            }
                        }
                    }
                }
                "enums" => {
                    let _s = info_span!("enum", node = node_span_field(&registry_child)).entered();
                    trace!("encountered node");
                    match registry_child.attribute("type") {
                        Some("enum") => registry.enums.push(Enum::from_node(registry_child, api)),
                        Some("bitmask") => registry
                            .bitmask_bits
                            .push(BitMaskBits::from_node(registry_child, api)),
                        Some("constants") => {
                            registry.constants.extend(
                                registry_child
                                    .children()
                                    .filter(|node| node.has_tag_name("enum"))
                                    .filter(|node| api_matches(node, api))
                                    .map(BaseConstant::from_node),
                            );
                        }
                        _ => trace!("ignored"),
                    }
                }
                "commands" => {
                    for command_node in registry_child
                        .children()
                        .filter(|node| node.has_tag_name("command"))
                        .filter(|node| api_matches(node, api))
                    {
                        let _s =
                            info_span!("command", node = node_span_field(&command_node)).entered();
                        trace!("encountered node");
                        if command_node.has_attribute("alias") {
                            registry
                                .command_aliases
                                .push(CommandAlias::from_node(command_node));
                        } else {
                            registry
                                .commands
                                .push(Command::from_node(command_node, api));
                        }
                    }
                }
                "feature" => {
                    let _s =
                        info_span!("feature", node = node_span_field(&registry_child)).entered();
                    trace!("encountered node");
                    registry
                        .features
                        .push(Feature::from_node(registry_child, library_name, api));
                }
                "extensions" => {
                    for extension_node in registry_child
                        .children()
                        .filter(|node| node.has_tag_name("extension"))
                        .filter(|node| {
                            node.attribute("supported")
                                .map(|values| values.split(',').any(|support| support == api))
                                .unwrap_or(true)
                        })
                    {
                        let _s = info_span!("extension", node = node_span_field(&extension_node))
                            .entered();
                        trace!("encountered node");
                        registry.extensions.push(Extension::from_node(
                            extension_node,
                            library_name,
                            api,
                        ));
                    }
                }
                _ => (),
            }
        }

        registry
    }
}

#[derive(Debug)]
pub struct TypeAlias {
    pub name: TypeName,
    pub alias: TypeName,
}

impl TypeAlias {
    fn from_node(node: Node) -> TypeAlias {
        TypeAlias {
            name: TypeName(attribute(node, "name").unwrap()),
            alias: TypeName(attribute(node, "alias").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct ConstantAlias {
    pub name: ConstantName,
    pub alias: ConstantName,
}

impl ConstantAlias {
    fn from_node(node: Node) -> ConstantAlias {
        ConstantAlias {
            name: ConstantName(attribute(node, "name").unwrap()),
            alias: ConstantName(attribute(node, "alias").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct CommandAlias {
    pub name: CommandName,
    pub alias: CommandName,
}

impl CommandAlias {
    fn from_node(node: Node) -> CommandAlias {
        CommandAlias {
            name: CommandName(attribute(node, "name").unwrap()),
            alias: CommandName(attribute(node, "alias").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct External {
    pub name: &'static str,
    pub requires: Option<&'static str>,
}

impl External {
    fn from_node(node: Node) -> External {
        External {
            name: attribute(node, "name").unwrap(),
            requires: attribute(node, "requires"),
        }
    }
}

#[derive(Debug)]
pub struct BaseType {
    pub name: TypeName,
    pub ty: &'static str,
}

impl BaseType {
    fn from_node(node: Node) -> Option<BaseType> {
        Some(BaseType {
            name: TypeName(child_text(node, "name").unwrap()),
            ty: child_text(node, "type")?,
        })
    }
}

#[derive(Debug)]
pub struct BitMask {
    pub requires: Option<&'static str>,
    pub bitvalues: Option<&'static str>,
    pub ty: &'static str,
    pub name: TypeName,
}

impl BitMask {
    fn from_node(node: Node) -> BitMask {
        BitMask {
            requires: attribute(node, "requires"),
            bitvalues: attribute(node, "bitvalues"),
            ty: child_text(node, "type").unwrap(),
            name: TypeName(child_text(node, "name").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct Handle {
    pub parent: Option<&'static str>,
    pub objtypeenum: &'static str,
    pub ty: &'static str,
    pub name: TypeName,
}

impl Handle {
    fn from_node(node: Node) -> Handle {
        Handle {
            parent: attribute(node, "parent"),
            objtypeenum: attribute(node, "objtypeenum").unwrap(),
            ty: child_text(node, "type").unwrap(),
            name: TypeName(child_text(node, "name").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct EnumType {
    pub name: &'static str,
}

impl EnumType {
    fn from_node(node: Node) -> EnumType {
        EnumType {
            name: attribute(node, "name").unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct FuncPointer {
    pub return_type: Option<CType<'static>>,
    pub name: FuncPointerName,
    pub params: Vec<CDecl<'static>>,
    pub requires: Option<&'static str>,
}

impl FuncPointer {
    fn from_node(node: Node, api: &str) -> FuncPointer {
        let proto = node
            .children()
            .find(|child| child.has_tag_name("proto"))
            .filter(|node| api_matches(node, api))
            .unwrap();

        // FIXME(eddyb) `CDeclMode::StructMember` should work but isn't accurate.
        let proto_cdecl = CDecl::from_xml(CDeclMode::StructMember, proto.children());
        FuncPointer {
            return_type: Some(proto_cdecl.ty).filter(|ty| *ty != CType::VOID),
            name: FuncPointerName(proto_cdecl.name),
            params: node
                .children()
                .filter(|child| child.has_tag_name("param"))
                .filter(|node| api_matches(node, api))
                .map(|node| CDecl::from_xml(CDeclMode::FuncParam, node.children()))
                .collect(),
            requires: attribute(node, "requires"),
        }
    }
}

#[derive(Debug)]
pub struct StructureMember {
    pub c_decl: CDecl<'static>,
    pub values: Option<&'static str>,
    pub len: Vec<&'static str>,
    pub altlen: Option<&'static str>,
    pub optional: Vec<&'static str>,
}

impl StructureMember {
    fn from_node(node: Node) -> StructureMember {
        StructureMember {
            c_decl: CDecl::from_xml(CDeclMode::StructMember, node.children()),
            values: attribute(node, "values"),
            len: attribute_comma_separated(node, "len"),
            altlen: attribute(node, "altlen"),
            optional: attribute_comma_separated(node, "optional"),
        }
    }
}

#[derive(Debug)]
pub struct Structure {
    pub name: TypeName,
    pub structextends: Vec<&'static str>,
    pub members: Vec<StructureMember>,
}

impl Structure {
    fn from_node(node: Node, api: &str) -> Structure {
        Structure {
            name: TypeName(attribute(node, "name").unwrap()),
            structextends: attribute_comma_separated(node, "structextends"),
            members: node
                .children()
                .filter(|node| node.has_tag_name("member"))
                .filter(|node| api_matches(node, api))
                .map(StructureMember::from_node)
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct CMacro {
    pub name: CMacroName,
    pub args: Vec<&'static str>,
    pub cexpr: CExprItems,
}

impl CMacro {
    fn from_node(node: Node) -> Option<CMacro> {
        #[derive(Debug, PartialEq)]
        enum Item {
            NameTag(&'static str),
            TypeTag(&'static str),
            Text(&'static str),
        }

        let mut items = node.children().flat_map(|child| match child.node_type() {
            NodeType::Text => text(child)
                .lines()
                .filter_map(|mut line| {
                    if let Some(comment) = line.find("//") {
                        line = &line[..comment];
                    }

                    let trimmed = line.trim_end_matches('\\').trim();
                    (!trimmed.is_empty()).then_some(Item::Text(trimmed))
                })
                .collect(),
            NodeType::Element => match child.tag_name().name() {
                "name" => vec![Item::NameTag(text(child))],
                "type" => vec![Item::TypeTag(text(child))],
                tag => unreachable!("unexpected `<{tag}>` in define"),
            },
            NodeType::Root | NodeType::PI | NodeType::Comment => unreachable!(),
        });

        if !matches!(items.next(), Some(Item::Text(t)) if t == "#define") {
            trace!("missing #define");
            return None;
        }

        let Some(Item::NameTag(name)) = items.next() else {
            unreachable!("define has no name");
        };

        if !name.contains("VERSION") {
            trace!("not a relevant macro (no VERSION in name)");
            return None;
        }

        fn eat_list(s: &mut &'static str) -> Option<impl Iterator<Item = &'static str>> {
            s.find(')') // naive, but works for us
                .filter(|_| s.starts_with('('))
                .map(|end| {
                    // `end` is before the closing brace
                    let (a, b) = s.split_at(end);
                    *s = &b[1..];
                    a[1..].split(',').map(str::trim)
                })
        }

        let mut cexpr = Vec::new();
        let mut args = None;
        let mut calling = None;
        for item in items {
            match item {
                Item::NameTag(..) => unreachable!(),
                Item::TypeTag(macro_name) => {
                    calling = Some(CMacroName(macro_name));
                    args = Some(vec![]);
                }
                Item::Text(mut s) => {
                    if args.is_none() {
                        args = Some(eat_list(&mut s).map(|i| i.collect()).unwrap_or_default());
                    } else if let Some(macro_name) = calling.take() {
                        let args = eat_list(&mut s)
                            .map(|i| i.map(cexpr::parse).collect())
                            .unwrap_or_default();
                        cexpr.push(CExprItem::MacroCall { macro_name, args });
                    }

                    cexpr.extend(cexpr::parse(s));
                }
            }
        }

        Some(CMacro {
            name: CMacroName(name),
            args: args?,
            cexpr,
        })
    }
}

#[derive(Debug)]
pub struct BaseConstant {
    pub ty: &'static str,
    pub value: CExprItems,
    pub name: ConstantName,
}

impl BaseConstant {
    fn from_node(node: Node) -> BaseConstant {
        BaseConstant {
            ty: attribute(node, "type").unwrap(),
            value: cexpr::parse(attribute(node, "value").unwrap()),
            name: ConstantName(attribute(node, "name").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct EnumValue {
    pub value: CExprItems,
    pub name: &'static str,
}

impl EnumValue {
    fn from_node(node: Node) -> EnumValue {
        EnumValue {
            value: cexpr::parse(attribute(node, "value").unwrap()),
            name: attribute(node, "name").unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct Enum {
    pub name: TypeName,
    pub values: Vec<EnumValue>,
    pub aliases: Vec<TypeAlias>,
}

impl Enum {
    fn from_node(node: Node, api: &str) -> Enum {
        let mut value = Enum {
            name: TypeName(attribute(node, "name").unwrap()),
            values: Vec::new(),
            aliases: Vec::new(),
        };

        for variant in node
            .children()
            .filter(|node| node.has_tag_name("enum"))
            .filter(|node| api_matches(node, api))
        {
            if variant.has_attribute("alias") {
                value.aliases.push(TypeAlias::from_node(variant));
            } else {
                value.values.push(EnumValue::from_node(variant));
            }
        }

        value
    }
}

#[derive(Debug)]
pub struct BitMaskBit {
    pub bitpos: &'static str,
    pub name: ConstantName,
}

impl BitMaskBit {
    fn from_node(node: Node) -> BitMaskBit {
        BitMaskBit {
            bitpos: attribute(node, "bitpos").unwrap(),
            name: ConstantName(attribute(node, "name").unwrap()),
        }
    }
}

#[derive(Debug)]
pub struct BitMaskBits {
    pub name: TypeName,
    pub bits: Vec<BitMaskBit>,
    /// Some bitmask variants represent literal values instead of specific
    /// individual bits, e.g. a combination of bits, or no bits at all. A good
    /// example for this is `VkCullModeFlagBits::FRONT_AND_BACK`.
    pub values: Vec<EnumValue>,
    pub aliases: Vec<ConstantAlias>,
}

impl BitMaskBits {
    fn from_node(node: Node, api: &str) -> BitMaskBits {
        let mut value = BitMaskBits {
            name: TypeName(attribute(node, "name").unwrap()),
            bits: Vec::new(),
            values: Vec::new(),
            aliases: Vec::new(),
        };

        for variant in node
            .children()
            .filter(|node| node.has_tag_name("enum"))
            .filter(|node| api_matches(node, api))
        {
            if variant.has_attribute("alias") {
                value.aliases.push(ConstantAlias::from_node(variant));
            } else if variant.has_attribute("value") {
                value.values.push(EnumValue::from_node(variant));
            } else {
                value.bits.push(BitMaskBit::from_node(variant));
            }
        }

        value
    }
}

#[derive(Debug)]
pub struct CommandParam {
    pub c_decl: CDecl<'static>,
    pub len: Option<&'static str>,
    pub altlen: Option<&'static str>,
    pub optional: Vec<&'static str>,
}

impl CommandParam {
    fn from_node(node: Node) -> CommandParam {
        CommandParam {
            c_decl: CDecl::from_xml(CDeclMode::FuncParam, node.children()),
            len: attribute(node, "len"),
            altlen: attribute(node, "altlen"),
            optional: attribute_comma_separated(node, "optional"),
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub return_type: Option<CType<'static>>,
    pub name: CommandName,
    pub params: Vec<CommandParam>,
}

impl Command {
    fn from_node(node: Node, api: &str) -> Command {
        let proto = node
            .children()
            .find(|child| child.has_tag_name("proto"))
            .filter(|node| api_matches(node, api))
            .unwrap();
        // FIXME(eddyb) `CDeclMode::StructMember` should work but isn't accurate.
        let proto_cdecl = CDecl::from_xml(CDeclMode::StructMember, proto.children());
        Command {
            return_type: Some(proto_cdecl.ty).filter(|ty| *ty != CType::VOID),
            name: CommandName(proto_cdecl.name),
            params: node
                .children()
                .filter(|child| child.has_tag_name("param"))
                .filter(|node| api_matches(node, api))
                .map(CommandParam::from_node)
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct RequireConstant {
    pub name: ConstantName,
    /// `Some` indicates a new constant being defined here.
    pub value: Option<CExprItems>,
}

impl RequireConstant {
    fn from_node(node: Node) -> RequireConstant {
        RequireConstant {
            name: ConstantName(attribute(node, "name").unwrap()),
            value: attribute(node, "value").map(cexpr::parse),
        }
    }
}

#[derive(Debug)]
pub struct RequireEnumExtendsOffset {
    pub offset: u32,
    pub extension_num: u32,
    pub positive_dir: bool,
}

impl RequireEnumExtendsOffset {
    pub fn resolve_value(&self) -> i32 {
        let ext_base = 1_000_000_000;
        let ext_block_size = 1000;
        let value = ext_base + (self.extension_num - 1) * ext_block_size + self.offset;

        if self.positive_dir {
            value as i32
        } else {
            -(value as i32)
        }
    }
}

#[derive(Debug)]
pub enum RequireEnumExtendsValue {
    Value(&'static str),
    BitPos(u8),
    Alias(ConstantName),
    Offset(RequireEnumExtendsOffset),
}

#[derive(Debug)]
pub struct RequireEnumExtends {
    pub name: &'static str,
    pub extends: &'static str,
    pub value: RequireEnumExtendsValue,
}

impl RequireEnumExtends {
    fn from_node(node: Node, extension_num: Option<u32>) -> RequireEnumExtends {
        RequireEnumExtends {
            name: attribute(node, "name").unwrap(),
            extends: attribute(node, "extends").unwrap(),
            value: if let Some(value) = attribute(node, "value") {
                RequireEnumExtendsValue::Value(value)
            } else if let Some(bitpos) = attribute(node, "bitpos").map(|v| v.parse().unwrap()) {
                RequireEnumExtendsValue::BitPos(bitpos)
            } else if let Some(alias) = attribute(node, "alias") {
                RequireEnumExtendsValue::Alias(ConstantName(alias))
            } else {
                RequireEnumExtendsValue::Offset(RequireEnumExtendsOffset {
                    offset: attribute(node, "offset").unwrap().parse().unwrap(),
                    extension_num: attribute(node, "extnumber")
                        .map(|v| v.parse().unwrap())
                        .or(extension_num)
                        .unwrap(),
                    positive_dir: !matches!(attribute(node, "dir"), Some("-")),
                })
            },
        }
    }
}

#[derive(Debug)]
pub enum RequireType {
    Type(TypeName),
    CMacro(CMacroName),
    FuncPointer(FuncPointerName),
    External(&'static str),
}

impl RequireType {
    fn from_node(node: Node, library_name: LibraryName) -> RequireType {
        let name = attribute(node, "name").unwrap();
        if name.starts_with("PFN_") {
            RequireType::FuncPointer(FuncPointerName(name))
        } else if name.starts_with(match library_name {
            LibraryName::Vk | LibraryName::Video => "VK_",
        }) {
            RequireType::CMacro(CMacroName(name))
        } else if name.starts_with(match library_name {
            LibraryName::Vk => "Vk",
            LibraryName::Video => "StdVideo",
        }) {
            RequireType::Type(TypeName(name))
        } else {
            RequireType::External(name)
        }
    }
}

#[derive(Debug)]
pub struct RequireCommand {
    pub name: CommandName,
}

impl RequireCommand {
    fn from_node(node: Node) -> RequireCommand {
        RequireCommand {
            name: CommandName(attribute(node, "name").unwrap()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    /// a string like "VK_COMPUTE", "VK_BASE", or "VK"
    pub api: &'static str,
    pub major: u32,
    pub minor: u32,
}

impl Version {
    fn from_str(s: &'static str) -> Option<Version> {
        let (api, major_minor) = s.split_once("_VERSION_")?;

        let mut iter = major_minor.split('_').flat_map(str::parse);
        let (Some(major), Some(minor), None) = (iter.next(), iter.next(), iter.next()) else {
            return None;
        };

        Some(Version { api, major, minor })
    }
}

#[derive(Debug, Default)]
pub struct Require {
    pub depends: Option<Depends>,
    pub enum_extends: Vec<RequireEnumExtends>,
    pub constants: Vec<RequireConstant>,
    pub types: Vec<RequireType>,
    pub commands: Vec<RequireCommand>,
}

impl Require {
    fn from_node(
        node: Node,
        library_name: LibraryName,
        api: &str,
        extension_num: Option<u32>,
    ) -> Require {
        let mut value = Require {
            depends: attribute(node, "depends").map(|input| Depends::from_str(input).unwrap()),
            ..Default::default()
        };

        for child in node.children().filter(|node| api_matches(node, api)) {
            match child.tag_name().name() {
                "enum" => {
                    if child.has_attribute("extends") {
                        value
                            .enum_extends
                            .push(RequireEnumExtends::from_node(child, extension_num));
                    } else {
                        value.constants.push(RequireConstant::from_node(child));
                    }
                }
                "type" => value
                    .types
                    .push(RequireType::from_node(child, library_name)),
                "command" => value.commands.push(RequireCommand::from_node(child)),
                _ => (),
            }
        }

        value
    }
}

#[derive(Debug)]
pub struct Feature {
    pub version: Version,
    pub depends: Option<Depends>,
    pub requires: Vec<Require>,
}

impl Feature {
    fn from_node(node: Node, library_name: LibraryName, api: &str) -> Feature {
        Feature {
            version: Version::from_str(attribute(node, "name").unwrap()).unwrap(),
            depends: attribute(node, "depends").map(|input| Depends::from_str(input).unwrap()),
            requires: node
                .children()
                .filter(|child| child.has_tag_name("require"))
                .filter(|node| api_matches(node, api))
                .map(|child| Require::from_node(child, library_name, api, None))
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct Extension {
    pub name: &'static str,
    pub number: Option<u32>,
    pub ty: Option<&'static str>,
    pub is_ratified: Option<bool>,
    pub depends: Option<Depends>,
    pub requires: Vec<Require>,
}

impl Extension {
    fn from_node(node: Node, library_name: LibraryName, api: &str) -> Extension {
        let extension_num = attribute(node, "number").map(|value| value.parse().unwrap());
        Extension {
            name: attribute(node, "name").unwrap(),
            number: extension_num,
            ty: attribute(node, "type"),
            is_ratified: matches!(library_name, LibraryName::Vk).then(|| {
                node.attribute("ratified")
                    .map(|values| values.split(',').any(|support| support == api))
                    .unwrap_or(false)
            }),
            depends: attribute(node, "depends").map(|input| Depends::from_str(input).unwrap()),
            requires: node
                .children()
                .filter(|child| child.has_tag_name("require"))
                .filter(|node| api_matches(node, api))
                .map(|child| Require::from_node(child, library_name, api, extension_num))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn vk_xml() {
        let xml_input = Box::leak(
            std::fs::read_to_string("../generator-rewrite/Vulkan-Headers/registry/vk.xml")
                .unwrap()
                .into_boxed_str(),
        );

        let waff3 = Registry::parse(xml_input, LibraryName::Vk, "vulkan");
        std::fs::write(
            "/home/friz64/source/ash/target/waff3",
            format!("{waff3:#?}"),
        )
        .unwrap();
    }

    #[test]
    #[traced_test]
    fn video_xml() {
        let xml_input = Box::leak(
            std::fs::read_to_string("../generator-rewrite/Vulkan-Headers/registry/video.xml")
                .unwrap()
                .into_boxed_str(),
        );

        let waff3 = Registry::parse(xml_input, LibraryName::Video, "vulkan");
        std::fs::write(
            "/home/friz64/source/ash/target/waff3video",
            format!("{waff3:#?}"),
        )
        .unwrap();
    }
}
