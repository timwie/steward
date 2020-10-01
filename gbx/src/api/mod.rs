pub use callbacks::*;
pub use calls::*;

mod callbacks;
mod calls;
pub mod structs;

/// The supported server API version.
///
/// Methods' and callbacks' signatures may differ across different versions.
pub const SERVER_API_VERSION: &str = "2013-04-16";

/// The supported script API version.
///
/// Script methods' and callbacks' signatures may differ across different versions.
pub const SCRIPT_API_VERSION: &str = "3.1.0";
