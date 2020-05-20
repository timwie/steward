pub use api::structs::*;
pub use api::{Callback, Calls, SCRIPT_API_VERSION, SERVER_API_VERSION};
pub use client::{RpcClient, RpcConnection};
pub use xml::{base64_encode, Fault};

mod adapter;
mod api;
mod client;
mod xml;
