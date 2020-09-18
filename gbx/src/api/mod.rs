pub use callbacks::*;
pub use calls::*;

mod callbacks;
mod calls;
pub mod structs;

/// The supported server API version.
///
/// Methods' and callbacks' signatures may differ across different versions.
///
/// Version history until 2013-04-16 is available in posts in the Dedicated Server forum at
/// https://forum.maniaplanet.com/viewforum.php?f=261
pub const SERVER_API_VERSION: &str = "2013-04-16";

/// The supported script API version.
///
/// Script methods' and callbacks' signatures may differ across different versions.
///
/// Version history up to 2.5.0 is available at https://github.com/maniaplanet/script-xmlrpc/releases
pub const SCRIPT_API_VERSION: &str = "3.1.0";
