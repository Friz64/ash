#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeName(&'static str);

impl TypeName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("Vk")
    }

    pub fn tag_trimmed(&self) -> &'static str {
        let original = self.original();
        let tag_boundary = original
            .rfind(|c: char| c.is_lowercase() || c.is_ascii_digit())
            .unwrap();
        &original[..=tag_boundary]
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ConstantName(&'static str);

impl ConstantName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EnumVariantName(&'static str);

impl EnumVariantName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CMacroName(&'static str);

impl CMacroName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FuncPointerName(&'static str);

impl FuncPointerName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn as_command_name(&self) -> CommandName {
        CommandName(self.0.strip_prefix("PFN_").unwrap())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CommandName(&'static str);

impl CommandName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VariableName(&'static str);

impl VariableName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }
}
