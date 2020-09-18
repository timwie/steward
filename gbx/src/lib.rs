pub use api::structs::*;
pub use api::*;
pub use client::{RpcClient, RpcConnection};
pub use xml::{base64_encode, Fault};

mod adapter;
mod api;
mod client;
pub mod file;
mod xml;
