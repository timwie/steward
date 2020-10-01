pub use api::*;
#[cfg(not(feature = "unit_test"))]
pub use client::*;
#[cfg(not(feature = "unit_test"))]
pub use xml::*;

#[cfg(not(feature = "unit_test"))]
mod adapter;
mod api;
#[cfg(not(feature = "unit_test"))]
mod client;
pub mod file;
#[cfg(not(feature = "unit_test"))]
mod xml;

#[cfg(feature = "unit_test")]
#[derive(Debug)]
pub struct Fault {
    pub code: i32,
    pub msg: String,
}
