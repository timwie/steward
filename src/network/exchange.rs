use chrono::NaiveDateTime;
use serde::Deserialize;
use thiserror::Error;

use crate::network::HTTP_CLIENT;
use crate::server::DisplayString;

/// Possible errors when querying Trackmania Exchange.
#[derive(Error, Debug)]
pub enum ExchangeError {
    /// Cannot find a map with the requested ID or UID.
    #[error("cannot find a trackmania.exchange map with the requested ID or UID")]
    UnknownId,

    /// The map is not available to download.
    #[error("this map is not downloadable")]
    NotDownloadable,

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
#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct ExchangeMetadata {
    /// The map's UID stored in its file.
    #[serde(rename = "TrackUID")]
    pub uid: String,

    /// The map's ID on the website (`https://trackmania.exchange/maps/<id>`)
    #[serde(rename = "TrackID")]
    pub exchange_id: i32,

    /// The formatted map name.
    pub name: DisplayString,

    /// The map author's in-game login.
    pub author_login: String,

    /// The map author's login on the Exchange website.
    #[serde(rename = "Username")]
    pub author_exchange_name: String,

    /// f.e. "2020-07-01T20:19:54.22"
    pub uploaded_at: NaiveDateTime,

    /// f.e. "2020-07-01T20:19:54.22"
    pub updated_at: NaiveDateTime,

    /// Not sure when maps are not downloadable.
    pub downloadable: bool,
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

    if !metadata.downloadable {
        return Err(ExchangeError::NotDownloadable);
    }

    let data = exchange_map_file(map_id).await?;
    Ok(ExchangeMap { metadata, data })
}

/// Fetch the Exchange ID of the specified map. Returns `Err(UnknownId)` if
/// that map is not on Exchange.
pub async fn exchange_id(map_id: &str) -> Result<i32, ExchangeError> {
    Ok(exchange_metadata(map_id).await?.exchange_id)
}

async fn exchange_metadata(map_id: &str) -> Result<ExchangeMetadata, ExchangeError> {
    let info_url = "https://trackmania.exchange/api/maps/get_map_info/multi/".to_string() + map_id;

    log::debug!("fetch exchange metadata for map id {}", map_id);
    let json: String = HTTP_CLIENT.get(&info_url).send().await?.text().await?;

    let metadata: ExchangeMetadata = serde_json::from_str::<Vec<ExchangeMetadata>>(&json)?
        .into_iter()
        .next()
        .ok_or(ExchangeError::UnknownId)?;

    Ok(metadata)
}

async fn exchange_map_file(map_id: &str) -> Result<Vec<u8>, ExchangeError> {
    let dl_url = "https://trackmania.exchange/tracks/download/".to_string() + map_id;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exchange_info() {
        let response = r#"
        {
           "TrackID":12,
           "UserID":3753,
           "Username":"MrLag",
           "UploadedAt":"2020-07-01T20:19:54.22",
           "UpdatedAt":"2020-07-01T20:19:54.22",
           "Name":"U-Turn",
           "TypeName":null,
           "MapType":"TM_Race",
           "TitlePack":"Trackmania",
           "Hide":false,
           "StyleName":"Race",
           "Mood":"48x48Sunrise",
           "DisplayCost":3727,
           "ModName":null,
           "Lightmap":8,
           "ExeVersion":"3.3.0",
           "ExeBuild":"2020-06-30_00_13",
           "EnvironmentName":"Stadium",
           "VehicleName":"",
           "UnlimiterRequired":false,
           "RouteName":null,
           "LengthName":"",
           "Laps":1,
           "DifficultyName":null,
           "ReplayTypeName":"",
           "ReplayWRID":null,
           "ReplayCount":0,
           "TrackValue":0,
           "Comments":"",
           "Unlisted":false,
           "AwardCount":18,
           "CommentCount":6,
           "MappackID":0,
           "ReplayWRTime":null,
           "ReplayWRUserID":null,
           "ReplayWRUsername":"",
           "Unreleased":false,
           "Downloadable":true,
           "GbxMapName":"$i$sU-Turn",
           "RatingVoteCount":0,
           "RatingVoteAverage":0.0,
           "TrackUID":"",
           "HasScreenshot":false,
           "HasThumbnail":true,
           "HasGhostBlocks":true,
           "EmbeddedObjectsCount":0,
           "AuthorLogin":"XI5FrosOSS-oDKWA9zNxbw",
           "IsMP4":true,
           "SizeWarning":false,
           "InPLList":false,
           "Status":0,
           "Position":0,
           "Added":"0001-01-01T00:00:00",
           "AddedBy":0,
           "AddedByName":"",
           "FeatureComment":"",
           "FeaturePinned":false,
           "ParserVersion":1,
           "EmbeddedItemsSize":12
        }"#;
        assert!(serde_json::from_str::<ExchangeMetadata>(&response).is_ok())
    }
}
