use crate::LibraryName;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeName(&'static str);

impl TypeName {
    pub const VK_RESULT: Self = Self::new("VkResult");
    pub const VK_OBJECT_TYPE: Self = Self::new("VkObjectType");
    pub const VK_DEVICE: Self = Self::new("VkDevice");
    pub const VK_COMMAND_BUFFER: Self = Self::new("VkCommandBuffer");
    pub const VK_QUEUE: Self = Self::new("VkQueue");
    pub const VK_INSTANCE: Self = Self::new("VkInstance");
    pub const VK_PHYSICAL_DEVICE: Self = Self::new("VkPhysicalDevice");
    pub const VK_STRUCTURE_TYPE: Self = Self::new("VkStructureType");
    pub const VK_BOOL32: Self = Self::new("VkBool32");

    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_stripped(&self, library: LibraryName) -> &'static str {
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
    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_stripped(&self, library: LibraryName) -> &'static str {
        self.original().trim_start_matches(match library {
            LibraryName::Vk => "VK_",
            LibraryName::Video => "STD_VIDEO_",
        })
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct EnumeratorName(&'static str);

impl EnumeratorName {
    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CMacroName(&'static str);

impl CMacroName {
    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_stripped(&self) -> &'static str {
        self.original().strip_prefix("VK_").unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FuncPointerName(&'static str);

impl FuncPointerName {
    pub(crate) const fn new(original: &'static str) -> Self {
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

    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_stripped(&self) -> &'static str {
        self.original().strip_prefix("vk").unwrap()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VariableName(&'static str);

impl VariableName {
    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ExtensionName(&'static str);

impl ExtensionName {
    pub(crate) const fn new(original: &'static str) -> Self {
        Self(original)
    }

    pub const fn original(&self) -> &'static str {
        self.0
    }

    pub fn prefix_stripped(&self, library: LibraryName) -> &'static str {
        match library {
            LibraryName::Vk => self.original().strip_prefix("VK_").unwrap(),
            LibraryName::Video => self.original().strip_prefix("vulkan_video_").unwrap(),
        }
    }

    pub fn tag_and_name(&self, library: LibraryName) -> (&'static str, &'static str) {
        let prefix_stripped = self.prefix_stripped(library);
        match library {
            LibraryName::Vk => prefix_stripped.split_once('_').unwrap(),
            LibraryName::Video => ("", prefix_stripped),
        }
    }
}
