use crate::{Import, Method};
use analysis::name::{CommandName, TypeName, VariableName};
use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use std::collections::HashMap;

fn parse_name_attr(attr: &syn::Attribute) -> Option<syn::Ident> {
    if !attr.path().is_ident("name") {
        return None;
    }

    attr.parse_args::<syn::Ident>().ok()
}

fn parse_import_attr(attr: &syn::Attribute) -> Option<Import> {
    if !attr.path().is_ident("import") {
        return None;
    }

    let variant = attr.parse_args::<syn::Ident>().ok()?;
    variant.to_string().parse().ok()
}

#[derive(Debug)]
pub enum BuilderRule {
    Keep,
    Skip,
    Replace(Method),
}

#[derive(Debug)]
pub struct BuilderRules(HashMap<(TypeName, VariableName), BuilderRule>);

impl BuilderRules {
    pub fn new(definitions: &syn::File) -> BuilderRules {
        let mut output = HashMap::new();

        for item in &definitions.items {
            let syn::Item::Impl(item_impl) = item else {
                panic!();
            };

            let type_name = if let syn::Type::Path(self_ty) = &*item_impl.self_ty
                && let Some(ident) = self_ty.path.get_ident()
            {
                TypeName::new(ident.to_string().leak())
            } else {
                panic!();
            };

            for impl_item in &item_impl.items {
                let (attrs, sig, impl_item_fn) = match impl_item {
                    syn::ImplItem::Fn(item_fn) => {
                        (item_fn.attrs.clone(), item_fn.sig.clone(), Some(item_fn))
                    }
                    syn::ImplItem::Verbatim(tokens) => {
                        let item_fn: syn::TraitItemFn = syn::parse2(tokens.clone()).unwrap();
                        (item_fn.attrs, item_fn.sig, None)
                    }
                    _ => panic!(),
                };

                let mut imports = Vec::new();
                let mut attributes = TokenStream::new();
                let mut rule: Box<dyn Fn(Vec<Import>, TokenStream) -> BuilderRule> =
                    Box::new(|_, _| panic!("no rule specified"));
                for attr in attrs {
                    if attr.path().is_ident("keep") {
                        rule = Box::new(|_, _| BuilderRule::Keep);
                    } else if attr.path().is_ident("skip") {
                        rule = Box::new(|_, _| BuilderRule::Skip);
                    } else if attr.path().is_ident("replace") {
                        let impl_item_fn =
                            impl_item_fn.expect("function with replace attr should have body");

                        rule = Box::new(|imports, attributes| {
                            let mut content = TokenStream::new();
                            content.append_all(&impl_item_fn.block.stmts);
                            BuilderRule::Replace(Method {
                                imports,
                                attributes,
                                name_override: None,
                                inputs: impl_item_fn.sig.inputs.to_token_stream(),
                                output: impl_item_fn.sig.output.to_token_stream(),
                                content,
                            })
                        });
                    } else if let Some(import) = parse_import_attr(&attr) {
                        imports.push(import);
                    } else {
                        attr.to_tokens(&mut attributes)
                    }
                }

                let field_name = VariableName::new(sig.ident.to_string().leak());
                output.insert((type_name, field_name), rule(imports, attributes));
            }
        }

        BuilderRules(output)
    }

    pub fn get(&self, struct_name: TypeName, field_name: VariableName) -> Option<&'_ BuilderRule> {
        self.0.get(&(struct_name, field_name))
    }
}

#[derive(Debug)]
pub struct WrapperRules(HashMap<CommandName, Method>);

impl WrapperRules {
    pub fn new(definitions: &syn::File) -> WrapperRules {
        let mut output = HashMap::new();

        for item in &definitions.items {
            let syn::Item::Fn(item_fn) = item else {
                panic!()
            };

            let mut name = None;
            let mut imports = Vec::new();
            let mut attributes = TokenStream::new();
            for attr in &item_fn.attrs {
                if let Some(value) = parse_import_attr(attr) {
                    imports.push(value);
                } else if let Some(value) = parse_name_attr(attr) {
                    name = Some(value);
                } else {
                    attr.to_tokens(&mut attributes);
                }
            }

            let command_name = CommandName::new(item_fn.sig.ident.to_string().leak());
            let mut content = TokenStream::new();
            content.append_all(&item_fn.block.stmts);
            output.insert(
                command_name,
                Method {
                    imports,
                    attributes,
                    name_override: name,
                    inputs: item_fn.sig.inputs.to_token_stream(),
                    output: item_fn.sig.output.to_token_stream(),
                    content,
                },
            );
        }

        WrapperRules(output)
    }

    pub fn get(&self, command_name: CommandName) -> Option<&'_ Method> {
        self.0.get(&command_name)
    }
}
