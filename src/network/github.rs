use semver::Version;
use serde::Deserialize;
use thiserror::Error;

use crate::network::HTTP_CLIENT;

/// Possible errors when querying Github.
#[derive(Error, Debug)]
pub enum GithubError {
    /// Failed to find a single version tag.
    #[error("cannot find the latest controller version on Github")]
    NoVersionFound,

    /// Wrong endpoint, or maybe not available right now.
    #[error("Github API request failed")]
    RequestError(#[from] reqwest::Error),

    /// Likely a bug on our end.
    #[error("failed to parse Github API response")]
    ParseError(#[from] serde_json::Error),
}

/// A tag on Github. Every release of this controller will be marked
/// with a tag, f.e. 'v0.1.0'.
#[derive(Deserialize, Debug)]
struct GithubTag {
    pub name: String,
}

/// Fetch the most recent version of this controller.
pub async fn most_recent_controller_version() -> Result<Version, GithubError> {
    let endpoint = "https://api.github.com/repos/timwie/steward/tags";

    let json: String = HTTP_CLIENT.get(endpoint).send().await?.text().await?;

    serde_json::from_str::<Vec<GithubTag>>(&json)?
        .into_iter()
        .find_map(
            |tag| match Version::parse(tag.name.trim_start_matches('v')) {
                Ok(version) => Some(version),
                Err(_) => None,
            },
        )
        .map_or_else(|| Err(GithubError::NoVersionFound), Ok)
}
