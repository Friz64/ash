use analysis::name::{TypeName, VariableName};
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::collections::HashMap;

#[derive(Debug)]
pub struct BuilderMethod {
    pub inputs: TokenStream,
    pub output: TokenStream,
    pub block: TokenStream,
}

#[derive(Debug)]
pub enum BuilderRule {
    Keep,
    Skip,
    Replace(BuilderMethod),
}

#[derive(Debug)]
pub struct BuilderRules(HashMap<(TypeName, VariableName), BuilderRule>);

impl BuilderRules {
    pub fn new(definitions: &syn::File) -> BuilderRules {
        let mut output = HashMap::new();

        for item in &definitions.items {
            let syn::Item::Impl(item_impl) = item else {
                continue;
            };

            let type_name = if let syn::Type::Path(self_ty) = &*item_impl.self_ty
                && let Some(ident) = self_ty.path.get_ident()
            {
                TypeName::new(ident.to_string().leak())
            } else {
                panic!();
            };

            for impl_item in &item_impl.items {
                let (attrs, sig, method) = match impl_item {
                    syn::ImplItem::Fn(item_fn) => (
                        item_fn.attrs.clone(),
                        item_fn.sig.clone(),
                        Some(BuilderMethod {
                            inputs: item_fn.sig.inputs.to_token_stream(),
                            output: item_fn.sig.output.to_token_stream(),
                            block: item_fn.block.to_token_stream(),
                        }),
                    ),
                    syn::ImplItem::Verbatim(tokens) => {
                        let item_fn: syn::TraitItemFn = syn::parse2(tokens.clone()).unwrap();
                        (item_fn.attrs, item_fn.sig, None)
                    }
                    _ => panic!(),
                };

                let Ok([attr]) = <&[_; 1]>::try_from(attrs.as_slice()) else {
                    panic!("expected there to be exactly one attribute on the function");
                };

                let value = if attr.path().is_ident("keep") {
                    BuilderRule::Keep
                } else if attr.path().is_ident("skip") {
                    BuilderRule::Skip
                } else if attr.path().is_ident("replace") {
                    BuilderRule::Replace(method.unwrap())
                } else {
                    panic!("unexpected attribute");
                };

                let field_name = VariableName::new(sig.ident.to_string().leak());
                output.insert((type_name, field_name), value);
            }
        }

        BuilderRules(output)
    }

    pub fn get(&self, struct_name: TypeName, field_name: VariableName) -> Option<&'_ BuilderRule> {
        self.0.get(&(struct_name, field_name))
    }
}
