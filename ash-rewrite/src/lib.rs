#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod device;
mod entry;
mod instance;

#[allow(
    unused_parens,
    clippy::double_parens,
    non_camel_case_types,
    clippy::missing_transmute_annotations
)]
mod generated;
/// Type definitions for platform-specific external types
pub mod platform_types;

use alloc::vec::Vec;
use core::{mem, ptr};
pub use generated::*;

#[allow(clippy::wrong_self_convention)]
pub trait Handle: Sized {
    const TYPE: vk::ObjectType;
    fn as_raw(self) -> u64;
    fn from_raw(_: u64) -> Self;

    /// Returns whether the handle is a `NULL` value.
    ///
    /// # Example
    ///
    /// ```
    /// # use ash::vk::{Handle, Instance};
    /// let instance = Instance::null();
    /// assert!(instance.is_null());
    /// ```
    fn is_null(self) -> bool {
        self.as_raw() == 0
    }
}

pub trait RawPtr<T> {
    fn to_raw_ptr(self) -> *const T;
}

impl<T> RawPtr<T> for Option<&T> {
    fn to_raw_ptr(self) -> *const T {
        match self {
            Some(inner) => inner,
            None => ptr::null(),
        }
    }
}

pub trait RawMutPtr<T> {
    fn to_raw_mut_ptr(self) -> *mut T;
}

impl<T> RawMutPtr<T> for Option<&mut T> {
    fn to_raw_mut_ptr(self) -> *mut T {
        match self {
            Some(inner) => inner,
            None => ptr::null_mut(),
        }
    }
}

pub type VkResult<T> = Result<T, vk::Result>;

impl vk::Result {
    #[inline]
    pub fn result(self) -> VkResult<()> {
        self.result_with_success(())
    }

    #[inline]
    pub fn result_with_success<T>(self, v: T) -> VkResult<T> {
        match self {
            Self::SUCCESS => Ok(v),
            _ => Err(self),
        }
    }

    /// # Safety
///
    /// [`mem::MaybeUninit::assume_init`]'s safety rules apply
    /// if `self` is exactly equal to [`vk::Result::SUCCESS`], i.e. 0.
    #[inline]
    pub unsafe fn assume_init_on_success<T>(self, v: mem::MaybeUninit<T>) -> VkResult<T> {
        self.result().map(move |()| v.assume_init())
    }

    /// # Safety
    ///
    /// [`Vec::set_len`]'s safety rules apply
    /// if `self` is exactly equal to [`vk::Result::SUCCESS`], i.e. 0.
    #[inline]
    pub unsafe fn set_vec_len_on_success<T>(self, mut v: Vec<T>, len: usize) -> VkResult<Vec<T>> {
        self.result().map(move |()| {
            v.set_len(len);
            v
        })
    }
}

/// Repeatedly calls `f` until it does not return [`vk::Result::INCOMPLETE`] anymore, ensuring all
/// available data has been read into the vector.
///
/// See for example [`vkEnumerateInstanceExtensionProperties`]: the number of available items may
/// change between calls; [`vk::Result::INCOMPLETE`] is returned when the count increased (and the
/// vector is not large enough after querying the initial size), requiring Ash to try again.
///
/// [`vkEnumerateInstanceExtensionProperties`]: https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceExtensionProperties.html
pub(crate) unsafe fn read_into_uninitialized_vector<N: Copy + Default + TryInto<usize>, T>(
    f: impl Fn(&mut N, *mut T) -> vk::Result,
) -> VkResult<Vec<T>>
where
    <N as TryInto<usize>>::Error: core::fmt::Debug,
{
    loop {
        let mut count = N::default();
        f(&mut count, ptr::null_mut()).result()?;
        let mut data =
            Vec::with_capacity(count.try_into().expect("`N` failed to convert to `usize`"));

        let err_code = f(&mut count, data.as_mut_ptr());
        if err_code != vk::Result::INCOMPLETE {
            break err_code.set_vec_len_on_success(
                data,
                count.try_into().expect("`N` failed to convert to `usize`"),
            );
        }
    }
}

pub unsafe trait TaggedStructure<'a>: Sized {
    const STRUCTURE_TYPE: StructureType;
}

pub unsafe trait Extends<B> {}

pub use device::*;
pub use entry::*;
pub use instance::*;
pub use vk1_0::DeviceFnV1_0;
pub use vk1_0::EntryFnV1_0;
pub use vk1_0::InstanceFnV1_0;
pub use vk1_0::StaticFn;
pub use vk1_1::DeviceFnV1_1;
pub use vk1_1::EntryFnV1_1;
pub use vk1_1::InstanceFnV1_1;
pub use vk1_2::DeviceFnV1_2;
pub use vk1_3::DeviceFnV1_3;
pub use vk1_3::InstanceFnV1_3;

use self::generated::vk::StructureType;

