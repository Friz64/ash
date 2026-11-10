use crate::{Function, Safety};
use analysis::name::{CommandName, TypeName, VariableName};
use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use std::collections::{HashMap, hash_map::Entry};

fn map_safety(safety: &syn::Safety) -> Safety {
    match safety {
        syn::Safety::Safe(_) => Safety::Safe,
        syn::Safety::Unsafe(_) => Safety::Unsafe,
        syn::Safety::Default => Safety::Default,
    }
}

#[derive(Debug)]
pub enum BuilderRule {
    Keep,
    Skip,
    Replace(Vec<Function>),
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

                let mut attributes = TokenStream::new();
                let mut rule: Box<dyn FnOnce(TokenStream) -> BuilderRule> =
                    Box::new(|_| panic!("no rule specified"));
                for attr in attrs {
                    if attr.path().is_ident("keep") {
                        rule = Box::new(|_attributes| BuilderRule::Keep);
                    } else if attr.path().is_ident("skip") {
                        rule = Box::new(|_attributes| BuilderRule::Skip);
                    } else if attr.path().is_ident("replace") {
                        let name_override = attr.parse_args::<syn::Ident>().ok();
                        let impl_item_fn =
                            impl_item_fn.expect("function with replace attr should have body");

                        rule = Box::new(move |attributes| {
                            let mut content = TokenStream::new();
                            content.append_all(&impl_item_fn.block.stmts);
                            BuilderRule::Replace(vec![Function {
                                attributes,
                                safety: map_safety(&impl_item_fn.sig.safety),
                                name_override,
                                inputs: impl_item_fn.sig.inputs.to_token_stream(),
                                output: impl_item_fn.sig.output.to_token_stream(),
                                content,
                            }])
                        });
                    } else {
                        attr.to_tokens(&mut attributes);
                    }
                }

                let field_name = VariableName::new(sig.ident.to_string().leak());
                let rule = rule(attributes);
                match output.entry((type_name, field_name)) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(rule);
                    }
                    Entry::Occupied(mut occupied) => {
                        if let BuilderRule::Replace(mut src) = rule
                            && let BuilderRule::Replace(dst) = occupied.get_mut()
                        {
                            dst.append(&mut src);
                        } else {
                            panic!("rule already exists");
                        }
                    }
                }
            }
        }

        BuilderRules(output)
    }

    pub fn get(&self, struct_name: TypeName, field_name: VariableName) -> Option<&'_ BuilderRule> {
        self.0.get(&(struct_name, field_name))
    }
}

#[derive(Debug)]
pub struct WrapperRules(HashMap<CommandName, Vec<Function>>);

impl WrapperRules {
    pub fn new(definitions: &syn::File) -> WrapperRules {
        let mut output: HashMap<CommandName, Vec<Function>> = HashMap::new();
        for item in &definitions.items {
            let syn::Item::Fn(item_fn) = item else {
                panic!()
            };

            let mut name = None;
            let mut attributes = TokenStream::new();
            for attr in &item_fn.attrs {
                if attr.path().is_ident("name")
                    && let Ok(value) = attr.parse_args::<syn::Ident>()
                {
                    name = Some(value);
                } else {
                    attr.to_tokens(&mut attributes);
                }
            }

            let command_name = CommandName::new(item_fn.sig.ident.to_string().leak());
            let mut content = TokenStream::new();
            content.append_all(&item_fn.block.stmts);
            output.entry(command_name).or_default().push(Function {
                attributes,
                safety: map_safety(&item_fn.sig.safety),
                name_override: name,
                inputs: item_fn.sig.inputs.to_token_stream(),
                output: item_fn.sig.output.to_token_stream(),
                content,
            });
        }

        WrapperRules(output)
    }

    pub fn get(&self, command_name: CommandName) -> Option<&'_ [Function]> {
        self.0.get(&command_name).map(|vec| vec.as_slice())
    }
}
