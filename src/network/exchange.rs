use serde::Deserialize;
use thiserror::Error;

use crate::network::HTTP_CLIENT;

/// Possible errors when querying Trackmania Exchange.
#[derive(Error, Debug)]
pub enum ExchangeError {
    /// Cannot find a map with the requested ID or UID.
    #[error("cannot find a trackmania.exchange map with the requested ID or UID")]
    UnknownId,

    /// Wrong endpoint, or maybe not available right now.
    #[error("trackmania.exchange API request failed")]
    RequestError(#[from] reqwest::Error),

    /// Likely a bug on our end.
    #[error("failed to parse trackmania.exchange API response")]
    ParseError(#[from] serde_json::Error),
}

/// Map information from Trackmania Exchange.
///
/// Reference: https://api.mania-exchange.com/documents/reference
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ExchangeMetadata {
    /// The map's UID stored in its file.
    #[serde(rename = "TrackUID")]
    pub uid: String,

    /// The map's ID on the website (`https://tm.mania-exchange.com/tracks/<id>`)
    #[serde(rename = "TrackID")]
    pub exchange_id: i32,

    /// The map name without its formatting.
    #[serde(rename = "Name")]
    pub name_plain: String,

    /// The formatted map name.
    #[serde(rename = "GbxMapName")]
    pub name: String,

    /// f.e. "Race"
    pub map_type: String,

    /// f.e. "Stadium"
    pub environment_name: String,
}

/// Map download from Trackmania Exchange.
pub struct ExchangeMap {
    /// Map metadata from the website.
    pub metadata: ExchangeMetadata,

    /// The map file.
    pub data: Vec<u8>,
}

/// Download a map from [trackmania.exchange](https://trackmania.exchange/).
///
/// The ID is either its ID on the website (a number), or
/// its UID (encoded in the GBX file's header).
pub async fn exchange_map(map_id: &str) -> Result<ExchangeMap, ExchangeError> {
    let metadata = exchange_metadata(map_id).await?;
    let data = exchange_map_file(map_id).await?;
    Ok(ExchangeMap { metadata, data })
}

/// Fetch the Exchange ID of the specified map. Returns `Err(UnknownId)` if
/// that map is not on Exchange.
pub async fn exchange_id(map_id: &str) -> Result<i32, ExchangeError> {
    Ok(exchange_metadata(map_id).await?.exchange_id)
}

async fn exchange_metadata(map_id: &str) -> Result<ExchangeMetadata, ExchangeError> {
    let info_url = "https://api.mania-exchange.com/tm/maps/".to_string() + map_id;

    log::debug!("fetch exchange metadata for map id {}", map_id);
    let json: String = HTTP_CLIENT.get(&info_url).send().await?.text().await?;

    let metadata: ExchangeMetadata = serde_json::from_str::<Vec<ExchangeMetadata>>(&json)?
        .into_iter()
        .next()
        .ok_or(ExchangeError::UnknownId)?;

    Ok(metadata)
}

async fn exchange_map_file(map_id: &str) -> Result<Vec<u8>, ExchangeError> {
    let dl_url = "https://tm.mania-exchange.com/tracks/download/".to_string() + map_id;

    log::debug!("fetch exchange file for map id {}", map_id);
    let data = HTTP_CLIENT
        .get(&dl_url)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();

    Ok(data)
}
