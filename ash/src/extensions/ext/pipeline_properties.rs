use crate::prelude::*;
use crate::vk;
use crate::{Device, Instance};
use std::ffi::CStr;
use std::mem;

/// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VK_EXT_pipeline_properties.html>
#[derive(Clone)]
pub struct PipelineProperties {
    handle: vk::Device,
    fp: vk::ExtPipelinePropertiesFn,
}

impl PipelineProperties {
    pub fn new(instance: &Instance, device: &Device) -> Self {
        let handle = device.handle();
        let fp = vk::ExtPipelinePropertiesFn::load(|name| unsafe {
            mem::transmute(instance.get_device_proc_addr(handle, name.as_ptr()))
        });
        Self { handle, fp }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkGetPipelinePropertiesEXT.html>
    ///
    /// TODO: Currently only accepts [`vk::PipelinePropertiesIdentifierEXT`]
    #[inline]
    pub unsafe fn get_pipeline_properties(
        &self,
        pipeline_info: &vk::PipelineInfoEXT,
        // TODO: https://github.com/krolli/vk-parse/pull/24
        pipeline_properties: *mut vk::BaseOutStructure,
    ) -> VkResult<()> {
        todo!("https://github.com/krolli/vk-parse/pull/24")
        // (self.fp.get_pipeline_properties_ext)(self.handle, pipeline_info, pipeline_properties)
        //     .result()
    }

    #[inline]
    pub const fn name() -> &'static CStr {
        vk::ExtPipelinePropertiesFn::name()
    }

    #[inline]
    pub fn fp(&self) -> &vk::ExtPipelinePropertiesFn {
        &self.fp
    }

    #[inline]
    pub fn device(&self) -> vk::Device {
        self.handle
    }
}
