//! This file describes "special cases" that are used to supplement the automatic wrapper generation
//! in cases where we want to integrate it into our loader abstraction or provide a "nicer" API.
//!
//! All function listed in this file will be used to replace the ones that would be auto-generated.
//!
//! Note that the command name must match the original one in vk.xml,
//! the generator will adjust it on its own accord.

#[deprecated = "This function is unavailable and therefore panics on Vulkan 1.0, please use `try_enumerate_instance_version()` instead"]
fn vkEnumerateInstanceVersion(&self) -> VkResult<u32> {
    let mut api_version = mem::MaybeUninit::uninit();
    (self.entry_fn_1_1.enumerate_instance_version)(api_version.as_mut_ptr())
        .assume_init_on_success(api_version)
}

/// # Safety
///
/// The resulting [`Instance`] and any function-pointer objects (e.g. [`Device`][crate::Device]
/// and extensions like [`crate::khr::swapchain::Device`]) loaded from it may not be used after
/// this [`Entry`] object is dropped, unless it was crated using [`Entry::linked()`] or
/// [`Entry::from_parts_1_1()`].
///
/// [`Instance`] does _not_ implement [drop][drop()] semantics and can only be destroyed via
/// [`destroy_instance()`][Instance::destroy_instance()].
fn vkCreateInstance(
    &self,
    create_info: &vk::InstanceCreateInfo<'_>,
    allocation_callbacks: Option<&vk::AllocationCallbacks>,
) -> VkResult<Instance> {
    let mut instance = mem::MaybeUninit::uninit();
    let instance = (self.entry_fn_1_0.create_instance)(
        create_info,
        allocation_callbacks.to_raw_ptr(),
        instance.as_mut_ptr(),
    )
    .assume_init_on_success(instance)?;
    Ok(Instance::load(&self.static_fn, instance))
}

fn vkGetInstanceProcAddr(
    &self,
    instance: vk::Instance,
    p_name: *const ffi::c_char,
) -> vk::PFN_vkVoidFunction {
    (self.static_fn.get_instance_proc_addr)(instance, p_name)
}

/// # Safety
///
/// There is a [parent/child relation] between [`Instance`] and the resulting [`Device`].  The
/// application must not [destroy][Instance::destroy_instance()] the parent [`Instance`] object
/// before first [destroying][Device::destroy_device()] the returned [`Device`] child object.
/// [`Device`] does _not_ implement [drop][drop()] semantics and can only be destroyed via
/// [`destroy_device()`][Device::destroy_device()].
///
/// See the [`Entry::create_instance()`] documentation for more destruction ordering rules on
/// [`Instance`].
///
/// [parent/child relation]: https://docs.vulkan.org/spec/latest/chapters/fundamentals.html#fundamentals-objectmodel-lifetime
fn vkCreateDevice(
    &self,
    physical_device: vk::PhysicalDevice,
    create_info: &vk::DeviceCreateInfo<'_>,
    allocation_callbacks: Option<&vk::AllocationCallbacks>,
) -> VkResult<Device> {
    let mut device = mem::MaybeUninit::uninit();
    let device = (self.instance_fn_1_0.create_device)(
        physical_device,
        create_info,
        allocation_callbacks.to_raw_ptr(),
        device.as_mut_ptr(),
    )
    .assume_init_on_success(device)?;
    Ok(Device::load(&self.instance_fn_1_0, device))
}

fn vkGetDeviceProcAddr(
    &self,
    device: vk::Device,
    p_name: *const ffi::c_char,
) -> vk::PFN_vkVoidFunction {
    (self.instance_fn_1_0.get_device_proc_addr)(device, p_name)
}

/// wip
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR() {}
/// wip
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceCountersByRegionARM() {}
/// wip
fn vkGetPipelineBinaryDataKHR() {}
/// wip
fn vkGetEncodedVideoSessionParametersKHR() {}
