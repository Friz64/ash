use crate::xml::Version;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Depends {
    Core(Version),
    Extension(&'static str),
    Feature {
        feature_structure: &'static str,
        feature_member: &'static str,
    },
    And(Box<Depends>, Box<Depends>),
    Or(Box<Depends>, Box<Depends>),
}

#[derive(Clone, Copy, Debug, Default)]
enum Mode {
    #[default]
    And,
    Or,
}

#[derive(Debug, Default)]
struct StackItem {
    mode: Mode,
    depends: Option<Depends>,
}

impl StackItem {
    fn extend(&mut self, new: Depends) {
        self.depends = Some(match (self.depends.take(), self.mode) {
            (None, _) => new,
            (Some(items), Mode::And) => Depends::And(Box::new(items), Box::new(new)),
            (Some(items), Mode::Or) => Depends::Or(Box::new(items), Box::new(new)),
        });
    }
}

#[derive(Debug, PartialEq)]
pub enum DependsParseError {
    NonAsciiInput,
    EmptySet,
    UnbalancedBrackets,
}

impl Depends {
    #[allow(clippy::should_implement_trait)] // we want a 'static input lifetime bc. of xml::Version
    pub fn from_str(input: &'static str) -> Result<Depends, DependsParseError> {
        if !input.is_ascii() {
            return Err(DependsParseError::NonAsciiInput);
        }

        let mut items = vec![StackItem::default()];
        fn top(items: &mut [StackItem]) -> Result<&mut StackItem, DependsParseError> {
            let top = items.last_mut();
            top.ok_or(DependsParseError::UnbalancedBrackets)
        }

        let is_punct = |c: char| matches!(c, '(' | ',' | '+' | ')');

        assert!(input.is_ascii());
        let mut s = input;
        while let Some(c) = s.chars().next() {
            if is_punct(c) {
                s = &s[1..];

                match c {
                    '(' => items.push(StackItem::default()),
                    ',' => top(&mut items)?.mode = Mode::Or,
                    '+' => top(&mut items)?.mode = Mode::And,
                    ')' => {
                        if items.len() <= 1 {
                            return Err(DependsParseError::UnbalancedBrackets);
                        }

                        let new =
                            (items.pop().unwrap().depends).ok_or(DependsParseError::EmptySet)?;
                        top(&mut items)?.extend(new);
                    }
                    _ => unreachable!(),
                }
            } else {
                let len = s.chars().take_while(|&c| !is_punct(c)).count();
                let (tok, rest) = s.split_at(len);
                s = rest;

                let new = if let Some(version) = Version::from_str(tok) {
                    Depends::Core(version)
                } else if let Some((feature_structure, feature_member)) = tok.split_once("::") {
                    Depends::Feature {
                        feature_structure,
                        feature_member,
                    }
                } else {
                    Depends::Extension(tok)
                };

                top(&mut items)?.extend(new);
            }
        }

        let [result]: [StackItem; 1] = items
            .try_into()
            .map_err(|_| DependsParseError::UnbalancedBrackets)?;
        result.depends.ok_or(DependsParseError::EmptySet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Depends::*;

    #[test]
    fn real_strings() {
        assert_eq!(
            Depends::from_str(
                "(((VK_KHR_bind_memory2+\
                    VK_KHR_get_physical_device_properties2+VK_KHR_sampler_ycbcr_conversion)\
                    ,VK_VERSION_1_1)+VK_KHR_image_format_list),VK_VERSION_1_2"
            ),
            Ok(Or(
                Box::new(And(
                    Box::new(Or(
                        Box::new(And(
                            Box::new(And(
                                Box::new(Extension("VK_KHR_bind_memory2")),
                                Box::new(Extension("VK_KHR_get_physical_device_properties2")),
                            )),
                            Box::new(Extension("VK_KHR_sampler_ycbcr_conversion")),
                        )),
                        Box::new(Core(Version {
                            api: "VK",
                            major: 1,
                            minor: 1
                        })),
                    )),
                    Box::new(Extension("VK_KHR_image_format_list"))
                )),
                Box::new(Core(Version {
                    api: "VK",
                    major: 1,
                    minor: 2
                }))
            )),
        );

        assert_eq!(
            Depends::from_str(
                "(VK_KHR_dynamic_rendering,VK_VERSION_1_3)+(VK_KHR_maintenance5,VK_VERSION_1_4)"
            ),
            Ok(And(
                Box::new(Or(
                    Box::new(Extension("VK_KHR_dynamic_rendering")),
                    Box::new(Core(Version {
                        api: "VK",
                        major: 1,
                        minor: 3
                    }))
                )),
                Box::new(Or(
                    Box::new(Extension("VK_KHR_maintenance5")),
                    Box::new(Core(Version {
                        api: "VK",
                        major: 1,
                        minor: 4
                    }))
                )),
            )),
        );

        assert_eq!(
            Depends::from_str(
                "VK_KHR_fragment_shading_rate+\
                    VkPhysicalDeviceMeshShaderFeaturesEXT::primitiveFragmentShadingRateMeshShader"
            ),
            Ok(And(
                Box::new(Extension("VK_KHR_fragment_shading_rate")),
                Box::new(Feature {
                    feature_structure: "VkPhysicalDeviceMeshShaderFeaturesEXT",
                    feature_member: "primitiveFragmentShadingRateMeshShader"
                }),
            )),
        );
    }

    #[test]
    fn weird() {
        assert_eq!(
            Depends::from_str("🦊"),
            Err(DependsParseError::NonAsciiInput)
        );
        assert_eq!(Depends::from_str(""), Err(DependsParseError::EmptySet));
        assert_eq!(Depends::from_str("()"), Err(DependsParseError::EmptySet));
        assert_eq!(
            Depends::from_str("(aa"),
            Err(DependsParseError::UnbalancedBrackets)
        );
        assert_eq!(
            Depends::from_str("aa)"),
            Err(DependsParseError::UnbalancedBrackets)
        );
        assert_eq!(Depends::from_str("(((a)))"), Ok(Extension("a")));
        assert_eq!(
            Depends::from_str("(((((a,((((b,((((c)))))))))))))"),
            Ok(Or(
                Box::new(Extension("a")),
                Box::new(Or(Box::new(Extension("b")), Box::new(Extension("c")))),
            ))
        );
    }
}
