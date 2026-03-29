#[allow(unused_parens, clippy::double_parens)]
mod generated;
/// Type definitions for platform-specific external types
pub mod platform_types;

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
