//! This file describes "special cases" that are used to supplement the automatic wrapper generation
//! in cases where the information in the XML is not sufficient, we want to provide a "nicer" API,
//! or we want to integrate it into our loader abstraction.
//!
//! All function listed in this file will be used to replace the ones that would be auto-generated.
//!
//! Note that the command name must match the original one in vk.xml,
//! the generator will adjust it on its own accord. If this is not what you want,
//! you can use `#[name(<name override>)]`.
//!
//! Replacing an alias will not work. Make sure not to use alias names.
//! The generator will automatically handle aliases. Because the function-pointer table may differ,
//! you should use `self.__table` to access the field.

#[deprecated = "This function is unavailable and therefore panics on Vulkan 1.0, please use `try_enumerate_instance_version()` instead"]
fn vkEnumerateInstanceVersion(&self) -> VkResult<u32> {
    let mut api_version = MaybeUninit::uninit();
    (self.__table.enumerate_instance_version)(api_version.as_mut_ptr())
        .assume_init_on_success(api_version)
}

/// # Example
///
/// ```no_run
/// # use ash_rewrite::{Entry, vk};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #[cfg(feature = "linked")]
/// let entry = Entry::linked();
/// #[cfg(feature = "loaded")]
/// let entry = unsafe { Entry::load() }?;
/// match unsafe { entry.try_enumerate_instance_version() }? {
///     // Vulkan 1.1+
///     Some(version) => {
///         let major = vk::version_major(version);
///         let minor = vk::version_minor(version);
///         let patch = vk::version_patch(version);
///     },
///     // Vulkan 1.0
///     None => {},
/// }
/// # Ok(()) }
/// ```
#[name(try_enumerate_instance_version)]
fn vkEnumerateInstanceVersion(&self) -> VkResult<Option<u32>> {
    let enumerate_instance_version: Option<vk::PFN_vkEnumerateInstanceVersion> = {
        transmute((self.static_fn.get_instance_proc_addr)(
            vk::Instance::null(),
            c"vkEnumerateInstanceVersion".as_ptr(),
        ))
    };

    if let Some(enumerate_instance_version) = enumerate_instance_version {
        let mut api_version = MaybeUninit::uninit();
        (enumerate_instance_version)(api_version.as_mut_ptr())
            .assume_init_on_success(api_version)
            .map(Some)
    } else {
        Ok(None)
    }
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
) -> VkResult<crate::Instance> {
    let mut instance = MaybeUninit::uninit();
    let instance = (self.entry_fn_1_0.create_instance)(
        create_info,
        allocation_callbacks.map_or(ptr::null(), |s| s),
        instance.as_mut_ptr(),
    )
    .assume_init_on_success(instance)?;
    Ok(crate::Instance::load(&self.static_fn, instance))
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
) -> VkResult<crate::Device> {
    let mut device = MaybeUninit::uninit();
    let device = (self.instance_fn_1_0.create_device)(
        physical_device,
        create_info,
        allocation_callbacks.map_or(ptr::null(), |s| s),
        device.as_mut_ptr(),
    )
    .assume_init_on_success(device)?;
    Ok(crate::Device::load(&self.instance_fn_1_0, device))
}

fn vkGetDeviceProcAddr(
    &self,
    device: vk::Device,
    p_name: *const ffi::c_char,
) -> vk::PFN_vkVoidFunction {
    (self.instance_fn_1_0.get_device_proc_addr)(device, p_name)
}

fn vkGetFenceStatus(&self, fence: vk::Fence) -> VkResult<bool> {
    let err_code = (self.__table.get_fence_status)(self.handle(), fence);
    match err_code {
        vk::Result::SUCCESS => Ok(true),
        vk::Result::NOT_READY => Ok(false),
        _ => Err(err_code),
    }
}

fn vkCmdBuildAccelerationStructuresKHR(
    &self,
    command_buffer: vk::CommandBuffer,
    infos: &[vk::AccelerationStructureBuildGeometryInfoKHR<'_>],
    build_range_infos: &[Option<&[vk::AccelerationStructureBuildRangeInfoKHR]>],
) {
    assert_eq!(infos.len(), build_range_infos.len());

    let build_range_infos = build_range_infos
        .iter()
        .zip(infos.iter())
        .map(|(range_info, info)| {
            if let Some(range_info) = range_info {
                assert_eq!(range_info.len(), info.geometry_count as usize);
                range_info.as_ptr()
            } else {
                // Must be NULL for at least VUID-vkCmdBuildAccelerationStructuresKHR-ppBuildRangeInfos-11544
                ptr::null()
            }
        })
        .collect::<Vec<_>>();

    (self.__table.cmd_build_acceleration_structures)(
        command_buffer,
        infos.len() as _,
        infos.as_ptr(),
        build_range_infos.as_ptr(),
    )
}

fn vkCmdBuildAccelerationStructuresIndirectKHR(
    &self,
    command_buffer: vk::CommandBuffer,
    infos: &[vk::AccelerationStructureBuildGeometryInfoKHR<'_>],
    indirect_device_addresses: &[vk::DeviceAddress],
    indirect_strides: &[u32],
    max_primitive_counts: &[&[u32]],
) {
    assert_eq!(infos.len(), indirect_device_addresses.len());
    assert_eq!(infos.len(), indirect_strides.len());
    assert_eq!(infos.len(), max_primitive_counts.len());

    let max_primitive_counts = max_primitive_counts
        .iter()
        .zip(infos.iter())
        .map(|(cnt, info)| {
            assert_eq!(cnt.len(), info.geometry_count as usize);
            cnt.as_ptr()
        })
        .collect::<Vec<_>>();

    (self.__table.cmd_build_acceleration_structures_indirect)(
        command_buffer,
        infos.len() as _,
        infos.as_ptr(),
        indirect_device_addresses.as_ptr(),
        indirect_strides.as_ptr(),
        max_primitive_counts.as_ptr(),
    );
}

fn vkBuildAccelerationStructuresKHR(
    &self,
    deferred_operation: vk::DeferredOperationKHR,
    infos: &[vk::AccelerationStructureBuildGeometryInfoKHR<'_>],
    build_range_infos: &[&[vk::AccelerationStructureBuildRangeInfoKHR]],
) -> VkResult<()> {
    assert_eq!(infos.len(), build_range_infos.len());

    let build_range_infos = build_range_infos
        .iter()
        .zip(infos.iter())
        .map(|(range_info, info)| {
            assert_eq!(range_info.len(), info.geometry_count as usize);
            range_info.as_ptr()
        })
        .collect::<Vec<_>>();

    (self.__table.build_acceleration_structures)(
        self.handle,
        deferred_operation,
        infos.len() as _,
        infos.as_ptr(),
        build_range_infos.as_ptr(),
    )
    .result()
}

// `Handle` trait assumes the object type to be `vk::ObjectType`, this function is an outlier
fn vkDebugReportMessageEXT(
    &self,
    flags: vk::DebugReportFlagsEXT,
    object_type: vk::DebugReportObjectTypeEXT,
    object: u64,
    location: usize,
    message_code: i32,
    layer_prefix: Option<&CStr>,
    message: Option<&CStr>,
) {
    (self.__table.debug_report_message)(
        self.handle,
        flags,
        object_type,
        object,
        location,
        message_code,
        layer_prefix.map_or(ptr::null(), |s| s.as_ptr()),
        message.map_or(ptr::null(), |s| s.as_ptr()),
    )
}

fn vkAcquireNextImageKHR(
    &self,
    swapchain: vk::SwapchainKHR,
    timeout: u64,
    semaphore: vk::Semaphore,
    fence: vk::Fence,
) -> VkResult<(u32, bool)> {
    let mut index = MaybeUninit::uninit();
    let err_code = (self.__table.acquire_next_image)(
        self.handle,
        swapchain,
        timeout,
        semaphore,
        fence,
        index.as_mut_ptr(),
    );
    match err_code {
        vk::Result::SUCCESS => Ok((index.assume_init(), false)),
        vk::Result::SUBOPTIMAL_KHR => Ok((index.assume_init(), true)),
        _ => Err(err_code),
    }
}

fn vkQueuePresentKHR(
    &self,
    queue: vk::Queue,
    present_info: &vk::PresentInfoKHR<'_>,
) -> VkResult<bool> {
    let err_code = (self.__table.queue_present)(queue, present_info);
    match err_code {
        vk::Result::SUCCESS => Ok(false),
        vk::Result::SUBOPTIMAL_KHR => Ok(true),
        _ => Err(err_code),
    }
}

fn vkAllocateDescriptorSets(
    &self,
    allocate_info: &vk::DescriptorSetAllocateInfo<'_>,
) -> VkResult<Vec<vk::DescriptorSet>> {
    let mut desc_set = Vec::with_capacity(allocate_info.descriptor_set_count as usize);
    (self.__table.allocate_descriptor_sets)(self.handle(), allocate_info, desc_set.as_mut_ptr())
        .set_vec_len_on_success(desc_set, allocate_info.descriptor_set_count as usize)
}

fn vkAllocateCommandBuffers(
    &self,
    allocate_info: &vk::CommandBufferAllocateInfo<'_>,
) -> VkResult<Vec<vk::CommandBuffer>> {
    let mut buffers = Vec::with_capacity(allocate_info.command_buffer_count as usize);
    (self.__table.allocate_command_buffers)(self.handle(), allocate_info, buffers.as_mut_ptr())
        .set_vec_len_on_success(buffers, allocate_info.command_buffer_count as usize)
}

fn vkGetQueryPoolResults<T>(
    &self,
    query_pool: vk::QueryPool,
    first_query: u32,
    data: &mut [T],
    flags: vk::QueryResultFlags,
) -> VkResult<()> {
    let data_size = mem::size_of_val(data);
    (self.__table.get_query_pool_results)(
        self.handle(),
        query_pool,
        first_query,
        data.len() as u32,
        data_size,
        data.as_mut_ptr().cast(),
        mem::size_of::<T>() as _,
        flags,
    )
    .result()
}

fn vkGetAccelerationStructureBuildSizesKHR(
    &self,
    build_type: vk::AccelerationStructureBuildTypeKHR,
    build_info: &vk::AccelerationStructureBuildGeometryInfoKHR<'_>,
    max_primitive_counts: Option<&[u32]>,
    size_info: &mut vk::AccelerationStructureBuildSizesInfoKHR<'_>,
) {
    let max_primitive_counts = if let Some(c) = max_primitive_counts {
        assert_eq!(c.len(), build_info.geometry_count as usize);
        c.as_ptr()
    } else {
        // Must be NULL for at least VUID-vkGetAccelerationStructureBuildSizesKHR-pMaxPrimitiveCounts-11613
        ptr::null()
    };

    (self.__table.get_acceleration_structure_build_sizes)(
        self.handle,
        build_type,
        build_info,
        max_primitive_counts,
        size_info,
    )
}

fn vkGetPipelineBinaryDataKHR(
    &self,
    info: &vk::PipelineBinaryDataInfoKHR,
    pipeline_binary_key: &mut vk::PipelineBinaryKeyKHR,
) -> VkResult<Vec<u8>> {
    read_into_uninitialized_vector(move |count, data: *mut u8| {
        (self.__table.get_pipeline_binary_data)(
            self.handle,
            info,
            pipeline_binary_key,
            count,
            data.cast(),
        )
    })
}
/// Returns a tuple containing `(timestamps, max_deviation)`
fn vkGetCalibratedTimestampsKHR(
    &self,
    info: &[vk::CalibratedTimestampInfoEXT<'_>],
) -> VkResult<(Vec<u64>, u64)> {
    let mut timestamps = Vec::with_capacity(info.len());
    let mut max_deviation = MaybeUninit::uninit();
    let max_deviation = (self.__table.get_calibrated_timestamps)(
        self.handle,
        info.len() as u32,
        info.as_ptr(),
        timestamps.as_mut_ptr(),
        max_deviation.as_mut_ptr(),
    )
    .assume_init_on_success(max_deviation)?;
    timestamps.set_len(info.len());
    Ok((timestamps, max_deviation))
}

/// Retrieve the number of elements to pass to [`enumerate_physical_device_queue_family_performance_query_counters()`][Self::enumerate_physical_device_queue_family_performance_query_counters()]
#[name(enumerate_physical_device_queue_family_performance_query_counters_len)]
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR(
    &self,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
) -> VkResult<usize> {
    let mut count = mem::MaybeUninit::uninit();
    (self
        .__table
        .enumerate_physical_device_queue_family_performance_query_counters)(
        physical_device,
        queue_family_index,
        count.as_mut_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
    )
    .assume_init_on_success(count)
    .map(|c| c as usize)
}

/// Call [`enumerate_physical_device_queue_family_performance_query_counters_len()`][Self::enumerate_physical_device_queue_family_performance_query_counters_len()] to query the number of elements to pass to `out_counters` and `out_counter_descriptions`.
/// Be sure to [`Default::default()`]-initialize these elements and optionally set their `p_next` pointer.
fn vkEnumeratePhysicalDeviceQueueFamilyPerformanceQueryCountersKHR(
    &self,
    physical_device: vk::PhysicalDevice,
    queue_family_index: u32,
    out_counters: &mut [vk::PerformanceCounterKHR<'_>],
    out_counter_descriptions: &mut [vk::PerformanceCounterDescriptionKHR<'_>],
) -> VkResult<()> {
    assert_eq!(out_counters.len(), out_counter_descriptions.len());
    let mut count = out_counters.len() as u32;
    (self
        .__table
        .enumerate_physical_device_queue_family_performance_query_counters)(
        physical_device,
        queue_family_index,
        &mut count,
        out_counters.as_mut_ptr(),
        out_counter_descriptions.as_mut_ptr(),
    )
    .result()?;
    assert_eq!(count as usize, out_counters.len());
    assert_eq!(count as usize, out_counter_descriptions.len());
    Ok(())
}

/// Retrieve the length of the byte slice to pass to [`get_encoded_video_session_parameters()`][Self::get_encoded_video_session_parameters()].
#[name(get_encoded_video_session_parameters_len)]
fn vkGetEncodedVideoSessionParametersKHR(
    &self,
    session_parameters_info: &vk::VideoEncodeSessionParametersGetInfoKHR<'_>,
    feedback_info: Option<&mut vk::VideoEncodeSessionParametersFeedbackInfoKHR<'_>>,
) -> VkResult<usize> {
    let mut size = mem::MaybeUninit::uninit();
    (self.__table.get_encoded_video_session_parameters)(
        self.handle,
        session_parameters_info,
        match feedback_info {
            Some(p) => p,
            None => ptr::null_mut(),
        },
        size.as_mut_ptr(),
        ptr::null_mut(),
    )
    .assume_init_on_success(size)
}

/// Call [`get_encoded_video_session_parameters_len()`][Self::get_encoded_video_session_parameters_len()] to query the length of the byte slice to pass to `out`.
fn vkGetEncodedVideoSessionParametersKHR(
    &self,
    session_parameters_info: &vk::VideoEncodeSessionParametersGetInfoKHR<'_>,
    feedback_info: Option<&mut vk::VideoEncodeSessionParametersFeedbackInfoKHR<'_>>,
    out: &mut [mem::MaybeUninit<u8>],
) -> VkResult<()> {
    let mut size = out.len();
    (self.__table.get_encoded_video_session_parameters)(
        self.handle,
        session_parameters_info,
        match feedback_info {
            Some(p) => p,
            None => ptr::null_mut(),
        },
        &mut size,
        out.as_mut_ptr() as _,
    )
    .result()?;
    assert_eq!(size, out.len());
    Ok(())
}

fn vkGetDeviceFaultInfoEXT(
    &self,
    fault_counts: &mut vk::DeviceFaultCountsEXT,
    fault_info: Option<&mut vk::DeviceFaultInfoEXT>,
) -> VkResult<()> {
    (self.__table.get_device_fault_info)(
        self.handle,
        fault_counts,
        match fault_info {
            Some(p) => p,
            None => ptr::null_mut(),
        },
    )
    .result()
}
