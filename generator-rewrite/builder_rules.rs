/// This file describes "special cases" that are used to supplement the automatic wrapper generation
/// in cases where the information in the XML is not sufficient.
///
/// - `#[keep]` = Force the generation of a function that would otherwise have been omitted
///               because it is specified as a "len" of another member.
/// - `#[skip]` = Skip the generation of a function.
/// - `#[replace]` = Replace the arguments, return type, and content of a function.
///
/// Note that the struct and member names must match the original ones in vk.xml,
/// the generator will adjust these on its own accord.

impl VkPipelineViewportStateCreateInfo {
    // pViewports is allowed to be empty if the viewport state is empty
    #[keep]
    fn viewportCount();

    // Must match viewportCount
    #[keep]
    fn scissorCount();
}

impl VkDescriptorSetLayoutBinding {
    // descriptorCount is settable regardless of having pImmutableSamplers
    #[keep]
    fn descriptorCount();
}

impl VkFramebufferCreateInfo {
    // No ImageView attachments when VK_FRAMEBUFFER_CREATE_IMAGELESS_BIT is set
    #[keep]
    fn attachmentCount();
}

impl VkWriteDescriptorSet {
    // descriptorCount also describes descriptor length in pNext extension structures
    // https://github.com/ash-rs/ash/issues/806
    #[keep]
    fn descriptorCount();
}

impl VkShaderModuleCreateInfo {
    #[skip]
    fn codeSize();

    #[replace]
    fn pCode(mut self, code: &'a [u32]) -> Self {
        self.code_size = code.len() * 4;
        self.p_code = code.as_ptr();
        self
    }
}

impl VkAccelerationStructureVersionInfoKHR {
    #[replace]
    fn pVersionData(mut self, version_data: &'a [u8; 2 * crate::vk::UUID_SIZE as usize]) -> Self {
        self.p_version_data = version_data.as_ptr();
        self
    }
}

impl VkMicromapVersionInfoEXT {
    #[replace]
    fn pVersionData(mut self, version_data: &'a [u8; 2 * crate::vk::UUID_SIZE as usize]) -> Self {
        self.p_version_data = version_data.as_ptr();
        self
    }
}
