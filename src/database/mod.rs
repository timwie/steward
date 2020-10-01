#[cfg(feature = "unit_test")]
pub use mock::*;
#[cfg(not(feature = "unit_test"))]
pub use queries::*;
pub use structs::*;

#[cfg(feature = "unit_test")]
mod mock;
#[cfg(not(feature = "unit_test"))]
mod queries;
mod structs;

#[derive(Clone)]
pub enum DatabaseClient {
    #[cfg(not(feature = "unit_test"))]
    Postgres(Pool),

    #[cfg(feature = "unit_test")]
    Mock(MockDatabase),
}
