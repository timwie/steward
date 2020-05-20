pub use callbacks::*;
pub use calls::*;
pub use structs::*;

mod callbacks;
mod calls;
pub mod structs;

/// The supported server API version.
///
/// Methods' and callbacks' signatures may differ across different versions.
///
/// Version history is available in posts in the Dedicated Server forum at
/// https://forum.maniaplanet.com/viewforum.php?f=261
pub const SERVER_API_VERSION: &str = "2019-10-23";

/// The supported script API version.
///
/// Script methods' and callbacks' signatures may differ across different versions.
///
/// Version history is available at https://github.com/maniaplanet/script-xmlrpc/releases
pub const SCRIPT_API_VERSION: &str = "2.5.0";
