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
    unreachable_patterns,
    clippy::missing_transmute_annotations,
    clippy::missing_safety_doc
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
    /// # use ash_rewrite::vk::{Handle, Instance};
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

#[cfg(feature = "std")]
impl std::error::Error for vk::Result {}

impl core::fmt::Display for vk::Result {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // TODO: don't be lazy and bring back the old functionality in the generator
        core::fmt::Debug::fmt(self, f)
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

#[cfg(feature = "debug")]
pub(crate) fn debug_flags<V: Into<u64> + Copy>(
    f: &mut core::fmt::Formatter<'_>,
    known: &[(V, &'static str)],
    value: V,
) -> core::fmt::Result {
    let mut first = true;
    let mut accum = value.into();
    for &(bit, name) in known {
        let bit = bit.into();
        if bit != 0 && accum & bit == bit {
            if !first {
                f.write_str(" | ")?;
            }
            f.write_str(name)?;
            first = false;
            accum &= !bit;
        }
    }
    if first || accum != 0 {
        if !first {
            f.write_str(" | ")?;
        }
        write!(f, "{accum:b}")?;
    }
    Ok(())
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

#[derive(Debug)]
pub struct CStrTooLargeForStaticArray {
    pub static_array_size: usize,
    pub c_str_size: usize,
}

impl core::error::Error for CStrTooLargeForStaticArray {}
impl core::fmt::Display for CStrTooLargeForStaticArray {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "static `c_char` target array of length `{}` is too small to write a `CStr` (with `NUL`-terminator) of length `{}`", self.static_array_size, self.c_str_size)
    }
}

pub(crate) fn write_c_str_slice_with_nul(
    target: &mut [core::ffi::c_char],
    str: &core::ffi::CStr,
) -> Result<(), CStrTooLargeForStaticArray> {
    let bytes = str.to_bytes_with_nul();
    // SAFETY: cast from c_char to u8 is ok because c_char is always one byte
    let bytes = unsafe { core::slice::from_raw_parts(bytes.as_ptr().cast(), bytes.len()) };
    let static_array_size = target.len();
    target
        .get_mut(..bytes.len())
        .ok_or(CStrTooLargeForStaticArray {
            static_array_size,
            c_str_size: bytes.len(),
        })?
        .copy_from_slice(bytes);
    Ok(())
}

pub(crate) fn wrap_c_str_slice_until_nul(
    str: &[core::ffi::c_char],
) -> Result<&core::ffi::CStr, core::ffi::FromBytesUntilNulError> {
    // SAFETY: The cast from c_char to u8 is ok because a c_char is always one byte.
    let bytes = unsafe { core::slice::from_raw_parts(str.as_ptr().cast(), str.len()) };
    core::ffi::CStr::from_bytes_until_nul(bytes)
}

/// Iterates through the pointer chain. Includes the item that is passed into the function. Stops at
/// the last [`vk::BaseOutStructure`] that has a null [`vk::BaseOutStructure::p_next`] field.
pub(crate) unsafe fn ptr_chain_iter<'a, T: TaggedStructure<'a>>(
    ptr: &mut T,
) -> impl Iterator<Item = *mut vk::BaseOutStructure<'_>> {
    let ptr = <*mut T>::cast::<vk::BaseOutStructure<'_>>(ptr);
    (0..).scan(ptr, |p_ptr, _| {
        if p_ptr.is_null() {
            return None;
        }
        let n_ptr = (**p_ptr).p_next;
        let old = *p_ptr;
        *p_ptr = n_ptr;
        Some(old)
    })
}

/// # Safety
///
/// Structures implementing this trait are layout-compatible with [`vk::BaseInStructure`] and
/// [`vk::BaseOutStructure`]. Such structures have an `s_type` field indicating its type, which must
/// always match the value of [`TaggedStructure::STRUCTURE_TYPE`].
pub unsafe trait TaggedStructure<'a>: Sized {
    const STRUCTURE_TYPE: StructureType;

    /// Prepends the given extension struct between the root and the first pointer. This method is
    /// only available on structs that can be passed to a function directly. Only valid extension
    /// structs can be pushed into the chain.
    /// If the chain looks like `A -> B -> C`, and you call `A.push(&mut D)`, then the
    /// chain will look like `A -> D -> B -> C`.
    ///
    /// # Panics
    /// If `next` contains a pointer chain of its own, this function will panic.  Call `unsafe`
    /// [`Self::extend()`] to insert this chain instead.
    fn push<'b: 'a, T: Extends<Self> + TaggedStructure<'b>>(mut self, next: &'a mut T) -> Self {
        // SAFETY: All implementers of `TaggedStructure` are required to have the `BaseOutStructure` layout
        let slf_base = unsafe { &mut *<*mut _>::cast::<vk::BaseOutStructure<'_>>(&mut self) };
        // SAFETY: All implementers of `T: TaggedStructure` are required to have the `BaseOutStructure` layout
        let next_base = unsafe { &mut *<*mut T>::cast::<vk::BaseOutStructure<'_>>(next) };
        // `next` here can contain a pointer chain.  This function refuses to insert the struct,
        // in favour of calling unsafe extend().
        assert!(
            next_base.p_next.is_null(),
            "push() expects a struct without an existing p_next pointer chain (equal to NULL)"
        );
        next_base.p_next = slf_base.p_next;
        slf_base.p_next = next_base;
        self
    }

    /// Prepends the given extension struct between the root and the first pointer. This method is
    /// only available on structs that can be passed to a function directly. Only valid extension
    /// structs can be pushed into the chain.
    /// If the chain looks like `A -> B -> C` and `D -> E`, and you call `A.extend(&mut D)`,
    /// then the chain will look like `A -> D -> E -> B -> C`.
    ///
    /// # Safety
    /// This function will walk the [`vk::BaseOutStructure::p_next`] chain of `next`, requiring
    /// all non-`NULL` pointers to point to a valid Vulkan structure starting with the
    /// [`vk::BaseOutStructure`] layout.
    ///
    /// The last struct in this chain (i.e. the one where `p_next` is `NULL`) must be writable
    /// memory, as its `p_next` field will be updated with the value of `self.p_next`.
    unsafe fn extend<'b: 'a, T: Extends<Self> + TaggedStructure<'b>>(
        mut self,
        next: &'a mut T,
    ) -> Self {
        // `next` here can contain a pointer chain. This means that we must correctly attach he head
        // to the root and the tail to the rest of the chain For example:
        //
        // next = A -> B
        // Before: `Root -> C -> D -> E`
        // After: `Root -> A -> B -> C -> D -> E`
        //                 ^^^^^^
        //                 next chain
        let slf_base = unsafe { &mut *<*mut _>::cast::<vk::BaseOutStructure<'_>>(&mut self) };
        let next_base = <*mut T>::cast::<vk::BaseOutStructure<'_>>(next);
        let last_next = ptr_chain_iter(next).last().unwrap();
        (*last_next).p_next = slf_base.p_next;
        slf_base.p_next = next_base;
        self
    }
}

/// Implemented for every structure that extends base structure `B`. Concretely that means struct
/// `B` is listed in its array of [`structextends` in the Vulkan registry][1].
///
/// # Safety
///
/// Similar to [`TaggedStructure`], all `unsafe` implementers of this trait must guarantee that
/// their structure is layout-compatible [`vk::BaseInStructure`] and [`vk::BaseOutStructure`].
///
/// [1]: https://registry.khronos.org/vulkan/specs/latest/styleguide.html#extensions-interactions
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
pub use vk1_4::DeviceFnV1_4;

use self::generated::vk::StructureType;
