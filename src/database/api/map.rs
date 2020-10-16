use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::database::Result;
use crate::server::DisplayString;

/// Database map.
#[derive(Clone, Debug)]
pub struct Map {
    /// Unique identifier.
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,

    /// The formatted map name.
    pub name: DisplayString,

    /// The map author's login.
    pub author_login: String,

    /// The map author's display name.
    ///
    /// Since TMNext this is the UPlay username, but formatted names might be possible
    /// again at some point.
    pub author_display_name: DisplayString,

    /// The "author time" in milliseconds. This is the time the map
    /// was validated with in the map editor.
    pub author_millis: i32,

    /// The moment this map was added to the database.
    pub added_since: NaiveDateTime,

    /// This map's ID on Trackmania Exchange, or `None` if it is unknown.
    pub exchange_id: Option<i32>,
}

/// A map that is in the database, but was deleted on the file system.
#[derive(Debug)]
pub struct RemovedMap {
    /// Unique identifier.
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`,
    /// or `None` if that file no longer exists.
    pub file_name: Option<String>,

    /// The formatted map name.
    pub name: DisplayString,

    /// The map author's login.
    pub author_login: String,

    /// The map author's display name.
    ///
    /// Since TMNext this is the UPlay username, but formatted names might be possible
    /// again at some point.
    pub author_display_name: DisplayString,

    /// This map's ID on Trackmania Exchange, or `None` if it is unknown.
    pub exchange_id: Option<i32>,
}

#[async_trait]
pub trait MapQueries {
    /// Return the `*.Map.Gbx` file contents of the specified map.
    async fn map_file(&self, uid: &str) -> Result<Option<Vec<u8>>>;

    /// Return the specified maps.
    async fn maps(&self, map_uids: Vec<&str>) -> Result<Vec<Map>>;

    /// Return the specified map, or `None` if no such map exists in the database.
    async fn map(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Insert a map into the database.
    ///
    /// If this exact map (with the same UID) already exists in the database, update
    ///  - its file,
    ///  - its file path,
    ///  - its exchange ID.
    ///
    /// If a map at the same file path already exists in the database,
    /// the existing map will remain in the database with its path removed,
    /// and the given map will be inserted.
    async fn upsert_map(&self, metadata: &Map, data: Vec<u8>) -> Result<()>;

    /// Delete a map, its preferences, and its records.
    /// The data is lost forever.
    async fn delete_map(&self, map_uid: &str) -> Result<Option<RemovedMap>>;

    /// Returns maps that have no file name, and cannot be played by the server.
    async fn removed_maps(&self) -> Result<Vec<RemovedMap>>;
}
