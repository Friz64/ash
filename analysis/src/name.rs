use heck::ToShoutySnekCase;

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
pub struct EnumeratorName(&'static str);

impl EnumeratorName {
    pub fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn stripped(&self, bits_name: TypeName) -> String {
        let mut prefix = bits_name
            .tag_trimmed()
            .replace("FlagBits", "")
            .TO_SHOUTY_SNEK_CASE();

        // add _ before trailing number
        if prefix.ends_with(|c: char| c.is_ascii_digit()) {
            prefix.insert(prefix.len() - 1, '_');
        }

        prefix.push('_');

        let prefix_stripped = self.original().strip_prefix(&prefix).unwrap();
        prefix_stripped.replace("_BIT", "")
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
