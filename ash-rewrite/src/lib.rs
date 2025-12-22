mod generated;
/// Type definitions for platform-specific external types
pub mod platform_types;

pub use generated::*;

// funny lil temp todo placeholder hack thingy :3
pub struct External<const T: usize>;
