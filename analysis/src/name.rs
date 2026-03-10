#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeName(pub(crate) &'static str);

impl TypeName {
    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("Vk")
    }

    pub fn prefix_tag_trimmed(&self) -> &'static str {
        let prefix_trimmed = self.prefix_trimmed();
        let tag_boundary = prefix_trimmed
            .rfind(|c: char| c.is_lowercase() || c.is_ascii_digit())
            .unwrap();
        prefix_trimmed.split_at(tag_boundary).0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ConstantName(pub(crate) &'static str);

impl ConstantName {
    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CMacroName(pub(crate) &'static str);

impl CMacroName {
    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FuncPointerName(pub(crate) &'static str);

impl FuncPointerName {
    pub fn original(&self) -> &'static str {
        self.0
    }

    pub fn as_command_name(&self) -> CommandName {
        CommandName(self.0.strip_prefix("PFN_").unwrap())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CommandName(pub(crate) &'static str);

impl CommandName {
    pub fn original(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VariableName(pub(crate) &'static str);

impl VariableName {
    pub fn original(&self) -> &'static str {
        self.0
    }
}
