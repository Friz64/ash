pub use crate::vk1_0::StaticFn;

pub use crate::vk1_0::EntryFnV1_0;
pub use crate::vk1_1::EntryFnV1_1;

pub use crate::vk1_0::InstanceFnV1_0;
pub use crate::vk1_1::InstanceFnV1_1;
pub use crate::vk1_3::InstanceFnV1_3;

pub use crate::vk1_0::DeviceFnV1_0;
pub use crate::vk1_1::DeviceFnV1_1;
pub use crate::vk1_2::DeviceFnV1_2;
pub use crate::vk1_3::DeviceFnV1_3;
pub use crate::vk1_4::DeviceFnV1_4;

#[cfg(feature = "loaded")]
use libloading::Library;

#[cfg(feature = "loaded")]
pub use self::loaded::*;

#[cfg(feature = "loaded")]
mod loaded {
    use super::*;
    use core::fmt;

    #[derive(Debug)]
    #[cfg_attr(docsrs, doc(cfg(feature = "loaded")))]
    pub enum LoadingError {
        LibraryLoadFailure(libloading::Error),
        MissingEntryPoint(MissingEntryPoint),
    }

    impl fmt::Display for LoadingError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::LibraryLoadFailure(err) => fmt::Display::fmt(err, f),
                Self::MissingEntryPoint(err) => fmt::Display::fmt(err, f),
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for LoadingError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(match self {
                Self::LibraryLoadFailure(err) => err,
                Self::MissingEntryPoint(err) => err,
            })
        }
    }

    impl From<MissingEntryPoint> for LoadingError {
        fn from(err: MissingEntryPoint) -> Self {
            Self::MissingEntryPoint(err)
        }
    }
}

use crate::{vk, RawPtr, VkResult};
use core::{ffi, fmt, mem, ptr};

/// Holds the Vulkan functions independent of a particular instance
#[derive(Clone)]
pub struct Entry {
    pub(crate) static_fn: crate::StaticFn,
    pub(crate) entry_fn_1_0: crate::EntryFnV1_0,
    pub(crate) entry_fn_1_1: crate::EntryFnV1_1,
    #[cfg(feature = "loaded")]
    _lib_guard: Option<alloc::sync::Arc<Library>>,
}

impl Entry {
    /// Load default Vulkan library for the current platform
    ///
    /// Prefer this over [`linked()`][Self::linked()] when your application can gracefully handle
    /// environments that lack Vulkan support, and when the build environment might not have Vulkan
    /// development packages installed (e.g. the Vulkan SDK, or Ubuntu's `libvulkan-dev`).
    ///
    /// # Safety
    ///
    /// `dlopen`ing native libraries is inherently unsafe. The safety guidelines
    /// for [`Library::new()`] and [`Library::get()`] apply here.
    ///
    /// No Vulkan functions loaded directly or indirectly from this [`Entry`]
    /// may be called after it is [dropped][drop()].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ash_rewrite::{vk, Entry};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let entry = unsafe { Entry::load()? };
    /// let app_info = vk::ApplicationInfo {
    ///     api_version: vk::make_api_version(0, 1, 0, 0),
    ///     ..Default::default()
    /// };
    /// let create_info = vk::InstanceCreateInfo {
    ///     p_application_info: &app_info,
    ///     ..Default::default()
    /// };
    /// let instance = unsafe { entry.create_instance(&create_info, None)? };
    /// # Ok(()) }
    /// ```
    #[cfg(feature = "loaded")]
    #[cfg_attr(docsrs, doc(cfg(feature = "loaded")))]
    pub unsafe fn load() -> Result<Self, LoadingError> {
        #[cfg(windows)]
        const LIB_PATH: &str = "vulkan-1.dll";

        #[cfg(all(
            unix,
            not(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "android",
                target_os = "fuchsia",
                target_env = "ohos"
            ))
        ))]
        const LIB_PATH: &str = "libvulkan.so.1";

        #[cfg(any(target_os = "android", target_os = "fuchsia", target_env = "ohos"))]
        const LIB_PATH: &str = "libvulkan.so";

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        const LIB_PATH: &str = "libvulkan.dylib";

        unsafe { Self::load_from(LIB_PATH) }
    }

    /// Load entry points from a Vulkan loader linked at compile time
    ///
    /// Compared to [`load()`][Self::load()], this is infallible, but requires that the build
    /// environment have Vulkan development packages installed (e.g. the Vulkan SDK, or Ubuntu's
    /// `libvulkan-dev`), and prevents the resulting binary from starting in environments that do not
    /// support Vulkan.
    ///
    /// Note that instance/device functions are still fetched via `vkGetInstanceProcAddr` and
    /// `vkGetDeviceProcAddr` for maximum performance.
    ///
    /// Any Vulkan function acquired directly or indirectly from this [`Entry`] may be called after it
    /// is [dropped][drop()].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ash::{vk, Entry};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let entry = Entry::linked();
    /// let app_info = vk::ApplicationInfo {
    ///     api_version: vk::make_api_version(0, 1, 0, 0),
    ///     ..Default::default()
    /// };
    /// let create_info = vk::InstanceCreateInfo {
    ///     p_application_info: &app_info,
    ///     ..Default::default()
    /// };
    /// let instance = unsafe { entry.create_instance(&create_info, None)? };
    /// # Ok(()) }
    /// ```
    #[cfg(feature = "linked")]
    #[cfg_attr(docsrs, doc(cfg(feature = "linked")))]
    pub fn linked() -> Self {
        // Sound because we're linking to Vulkan, which provides a vkGetInstanceProcAddr that has
        // defined behavior in this use.
        unsafe {
            Self::from_static_fn(crate::StaticFn {
                get_instance_proc_addr: vkGetInstanceProcAddr,
            })
        }
    }

    /// Load Vulkan library at `path`
    ///
    /// # Safety
    ///
    /// `dlopen`ing native libraries is inherently unsafe. The safety guidelines
    /// for [`Library::new()`] and [`Library::get()`] apply here.
    ///
    /// No Vulkan functions loaded directly or indirectly from this [`Entry`]
    /// may be called after it is [dropped][drop()].
    #[cfg(feature = "loaded")]
    #[cfg_attr(docsrs, doc(cfg(feature = "loaded")))]
    pub unsafe fn load_from(path: impl AsRef<std::ffi::OsStr>) -> Result<Self, LoadingError> {
        let lib = Library::new(path)
            .map_err(LoadingError::LibraryLoadFailure)
            .map(alloc::sync::Arc::new)?;

        let static_fn = crate::StaticFn::load_checked(|name| {
            lib.get(name.to_bytes_with_nul())
                .map(|symbol| *symbol)
                .unwrap_or(ptr::null_mut())
        })?;

        Ok(Self {
            _lib_guard: Some(lib),
            ..Self::from_static_fn(static_fn)
        })
    }

    /// Load entry points based on an already-loaded [`crate::StaticFn`]
    ///
    /// # Safety
    ///
    /// `static_fn` must contain valid function pointers that comply with the semantics specified
    /// by Vulkan 1.0, which must remain valid for at least the lifetime of the returned [`Entry`].
    pub unsafe fn from_static_fn(static_fn: crate::StaticFn) -> Self {
        let load_fn = move |name: &ffi::CStr| {
            mem::transmute((static_fn.get_instance_proc_addr)(
                vk::Instance::null(),
                name.as_ptr(),
            ))
        };

        Self::from_parts_1_1(
            static_fn,
            crate::EntryFnV1_0::load(load_fn),
            crate::EntryFnV1_1::load(load_fn),
        )
    }

    #[inline]
    pub fn from_parts_1_1(
        static_fn: crate::StaticFn,
        entry_fn_1_0: crate::EntryFnV1_0,
        entry_fn_1_1: crate::EntryFnV1_1,
    ) -> Self {
        Self {
            static_fn,
            entry_fn_1_0,
            entry_fn_1_1,
            #[cfg(feature = "loaded")]
            _lib_guard: None,
        }
    }

    #[inline]
    pub fn static_fn(&self) -> &crate::StaticFn {
        &self.static_fn
    }

    /// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceVersion.html>
    ///
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
    #[inline]
    pub unsafe fn try_enumerate_instance_version(&self) -> VkResult<Option<u32>> {
        let enumerate_instance_version: Option<vk::PFN_vkEnumerateInstanceVersion> = {
            let name = ffi::CStr::from_bytes_with_nul_unchecked(b"vkEnumerateInstanceVersion\0");
            mem::transmute((self.static_fn.get_instance_proc_addr)(
                vk::Instance::null(),
                name.as_ptr(),
            ))
        };
        if let Some(enumerate_instance_version) = enumerate_instance_version {
            let mut api_version = mem::MaybeUninit::uninit();
            (enumerate_instance_version)(api_version.as_mut_ptr())
                .assume_init_on_success(api_version)
                .map(Some)
        } else {
            Ok(None)
        }
    }

    /// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkCreateInstance.html>
    ///
    /// # Safety
    ///
    /// The resulting [`Instance`] and any function-pointer objects (e.g. [`Device`][crate::Device]
    /// and extensions like [`crate::khr::swapchain::Device`]) loaded from it may not be used after
    /// this [`Entry`] object is dropped, unless it was crated using [`Entry::linked()`] or
    /// [`Entry::from_parts_1_1()`].
    ///
    /// [`Instance`] does _not_ implement [drop][drop()] semantics and can only be destroyed via
    /// [`destroy_instance()`][Instance::destroy_instance()].
    #[inline]
    pub unsafe fn create_instance(
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

    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkGetInstanceProcAddr.html>
    #[inline]
    pub unsafe fn get_instance_proc_addr(
        &self,
        instance: vk::Instance,
        p_name: *const ffi::c_char,
    ) -> vk::PFN_vkVoidFunction {
        (self.static_fn.get_instance_proc_addr)(instance, p_name)
    }
}

impl Entry {
    #[deprecated = "This function is unavailable and therefore panics on Vulkan 1.0, please use `try_enumerate_instance_version()` instead"]
    /// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceVersion.html>
    ///
    /// Please use [`try_enumerate_instance_version()`][Self::try_enumerate_instance_version()] instead.
    #[inline]
    pub unsafe fn enumerate_instance_version(&self) -> VkResult<u32> {
        let mut api_version = mem::MaybeUninit::uninit();
        (self.entry_fn_1_1.enumerate_instance_version)(api_version.as_mut_ptr())
            .assume_init_on_success(api_version)
    }
}

#[cfg(feature = "linked")]
#[cfg_attr(docsrs, doc(cfg(feature = "linked")))]
impl Default for Entry {
    #[inline]
    fn default() -> Self {
        Self::linked()
    }
}

impl crate::StaticFn {
    pub fn load_checked<F>(mut _f: F) -> Result<Self, MissingEntryPoint>
    where
        F: FnMut(&ffi::CStr) -> *const ffi::c_void,
    {
        Ok(Self {
            get_instance_proc_addr: unsafe {
                let cname = ffi::CStr::from_bytes_with_nul_unchecked(b"vkGetInstanceProcAddr\0");
                let val = _f(cname);
                if val.is_null() {
                    return Err(MissingEntryPoint);
                } else {
                    mem::transmute(val)
                }
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct MissingEntryPoint;
impl fmt::Display for MissingEntryPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cannot load `vkGetInstanceProcAddr` symbol from library")
    }
}
#[cfg(feature = "std")] // TODO: implement when error_in_core is stabilized
impl std::error::Error for MissingEntryPoint {}

#[cfg(feature = "linked")]
extern "system" {
    fn vkGetInstanceProcAddr(
        instance: vk::Instance,
        name: *const ffi::c_char,
    ) -> vk::PFN_vkVoidFunction;
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkInstance.html>
#[derive(Clone)]
pub struct Instance {
    pub(crate) handle: vk::Instance,

    pub(crate) instance_fn_1_0: crate::InstanceFnV1_0,
    pub(crate) instance_fn_1_1: crate::InstanceFnV1_1,
    pub(crate) instance_fn_1_3: crate::InstanceFnV1_3,
}

impl Instance {
    pub unsafe fn load(static_fn: &crate::StaticFn, instance: vk::Instance) -> Self {
        Self::load_with(
            |name| mem::transmute((static_fn.get_instance_proc_addr)(instance, name.as_ptr())),
            instance,
        )
    }

    pub unsafe fn load_with(
        mut load_fn: impl FnMut(&ffi::CStr) -> *const ffi::c_void,
        instance: vk::Instance,
    ) -> Self {
        Self::from_parts(
            instance,
            crate::InstanceFnV1_0::load(&mut load_fn),
            crate::InstanceFnV1_1::load(&mut load_fn),
            crate::InstanceFnV1_3::load(&mut load_fn),
        )
    }

    #[inline]
    pub fn from_parts(
        handle: vk::Instance,
        instance_fn_1_0: crate::InstanceFnV1_0,
        instance_fn_1_1: crate::InstanceFnV1_1,
        instance_fn_1_3: crate::InstanceFnV1_3,
    ) -> Self {
        Self {
            handle,

            instance_fn_1_0,
            instance_fn_1_1,
            instance_fn_1_3,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Instance {
        self.handle
    }

    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkCreateDevice.html>
    ///
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
    #[inline]
    pub unsafe fn create_device(
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

    /// <https://docs.vulkan.org/refpages/latest/refpages/source/vkGetDeviceProcAddr.html>
    #[inline]
    pub unsafe fn get_device_proc_addr(
        &self,
        device: vk::Device,
        p_name: *const ffi::c_char,
    ) -> vk::PFN_vkVoidFunction {
        (self.instance_fn_1_0.get_device_proc_addr)(device, p_name)
    }
}

/// <https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkDevice.html>
#[derive(Clone)]
pub struct Device {
    pub(crate) handle: vk::Device,

    pub(crate) device_fn_1_0: crate::DeviceFnV1_0,
    pub(crate) device_fn_1_1: crate::DeviceFnV1_1,
    pub(crate) device_fn_1_2: crate::DeviceFnV1_2,
    pub(crate) device_fn_1_3: crate::DeviceFnV1_3,
    pub(crate) device_fn_1_4: crate::DeviceFnV1_4,
}

impl Device {
    pub unsafe fn load(instance_fn: &crate::InstanceFnV1_0, device: vk::Device) -> Self {
        Self::load_with(
            |name| mem::transmute((instance_fn.get_device_proc_addr)(device, name.as_ptr())),
            device,
        )
    }

    pub unsafe fn load_with(
        mut load_fn: impl FnMut(&ffi::CStr) -> *const ffi::c_void,
        device: vk::Device,
    ) -> Self {
        Self::from_parts(
            device,
            crate::DeviceFnV1_0::load(&mut load_fn),
            crate::DeviceFnV1_1::load(&mut load_fn),
            crate::DeviceFnV1_2::load(&mut load_fn),
            crate::DeviceFnV1_3::load(&mut load_fn),
            crate::DeviceFnV1_4::load(&mut load_fn),
        )
    }

    #[inline]
    pub fn from_parts(
        handle: vk::Device,
        device_fn_1_0: crate::DeviceFnV1_0,
        device_fn_1_1: crate::DeviceFnV1_1,
        device_fn_1_2: crate::DeviceFnV1_2,
        device_fn_1_3: crate::DeviceFnV1_3,
        device_fn_1_4: crate::DeviceFnV1_4,
    ) -> Self {
        Self {
            handle,

            device_fn_1_0,
            device_fn_1_1,
            device_fn_1_2,
            device_fn_1_3,
            device_fn_1_4,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Device {
        self.handle
    }
}
