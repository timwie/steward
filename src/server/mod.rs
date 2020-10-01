#[cfg(not(feature = "unit_test"))]
pub use gbx::RpcClient as Server;
pub use gbx::{Callback as ServerEvent, *};
#[cfg(feature = "unit_test")]
pub use mock::*;

#[cfg(feature = "unit_test")]
mod mock;
