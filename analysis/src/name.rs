use heck::ToShoutySnekCase;

use crate::LibraryName;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeName(&'static str);

impl TypeName {
    pub const VK_OBJECT_TYPE: Self = Self::new("VkObjectType");
    pub const VK_DEVICE: Self = Self::new("VkDevice");
    pub const VK_COMMAND_BUFFER: Self = Self::new("VkCommandBuffer");
    pub const VK_QUEUE: Self = Self::new("VkQueue");
    pub const VK_INSTANCE: Self = Self::new("VkInstance");
    pub const VK_PHYSICAL_DEVICE: Self = Self::new("VkPhysicalDevice");
    pub const VK_STRUCTURE_TYPE: Self = Self::new("VkStructureType");

    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self, library: LibraryName) -> &'static str {
        let prefix = match library {
            LibraryName::Vk => "Vk",
            LibraryName::Video => "StdVideo",
        };

        self.original().strip_prefix(prefix).unwrap()
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
    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().trim_start_matches("VK_")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EnumeratorName(&'static str);

impl EnumeratorName {
    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn stripped(&self, type_name: TypeName) -> String {
        let prefix = if type_name.original() == "VkResult" {
            String::from("VK_")
        } else {
            let mut prefix = type_name
                .tag_trimmed()
                .replace("FlagBits", "")
                .TO_SHOUTY_SNEK_CASE();

            // add _ before trailing number
            if prefix.ends_with(|c: char| c.is_ascii_digit()) {
                prefix.insert(prefix.len() - 1, '_');
            }

            prefix + "_"
        };

        let prefix_stripped = self.original().strip_prefix(&prefix).unwrap();
        prefix_stripped.replace("_BIT", "")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CMacroName(&'static str);

impl CMacroName {
    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().strip_prefix("VK_").unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FuncPointerName(&'static str);

impl FuncPointerName {
    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn as_command_name(&self) -> CommandName {
        CommandName(self.original().strip_prefix("PFN_").unwrap())
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CommandName(&'static str);

impl CommandName {
    pub const VK_GET_INSTANCE_PROC_ADDR: Self = Self::new("vkGetInstanceProcAddr");
    pub const VK_GET_DEVICE_PROC_ADDR: Self = Self::new("vkGetDeviceProcAddr");

    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_trimmed(&self) -> &'static str {
        self.original().strip_prefix("vk").unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VariableName(&'static str);

impl VariableName {
    pub const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }
}
