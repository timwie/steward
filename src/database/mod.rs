pub use postgres::pg_connect as db_connect;
pub use queries::Queries as Database;
pub use structs::*;

mod postgres;
mod queries;
pub mod structs;
