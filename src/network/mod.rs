use lazy_static::*;

pub use exchange::*;
pub use github::*;

use crate::constants::USER_AGENT;

mod exchange;
mod github;

lazy_static! {
    /// The client used for all HTTP requests.
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("failed to build http client");
}
