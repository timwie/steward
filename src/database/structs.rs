use chrono::{NaiveDateTime, Utc};
use postgres_types::{FromSql, ToSql};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::server::GameString;
use crate::server::MapInfo;

/// Database player that has joined the server at least once.
#[derive(Debug, PartialEq)]
pub struct Player {
    /// Player login.
    pub login: String,

    /// Formatted nick name.
    pub nick_name: GameString,
}

/// Stores the most recent time a player has played a specific map.
#[derive(Debug, PartialEq)]
pub struct History {
    pub player_login: String,
    pub map_uid: String,

    /// The time this player last played this map, or `None` if they have never played it.
    pub last_played: Option<NaiveDateTime>,

    /// The number of other maps played since `last_played`, which is a value in
    /// `0..nb_total_maps`.
    pub nb_maps_since: usize,
}

/// Database map.
#[derive(Clone, Debug)]
pub struct Map {
    /// Unique identifier.
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,

    /// The formatted map name.
    pub name: GameString,

    /// The map author's login.
    pub author_login: String,

    /// The "author time" in milliseconds. This is the time the map
    /// was validated with in the map editor.
    pub author_millis: i32,

    /// The moment this map was added to the database.
    pub added_since: NaiveDateTime,

    /// `True` if the map is in the server's playlist.
    pub in_playlist: bool,

    /// This map's ID on Trackmania Exchange, or `None` if it is unknown.
    pub exchange_id: Option<i32>,
}

impl From<MapInfo> for Map {
    fn from(info: MapInfo) -> Self {
        Map {
            uid: info.uid,
            file_name: info.file_name,
            name: info.name,
            author_login: info.author_login,
            added_since: Utc::now().naive_utc(),
            author_millis: info.author_millis,
            in_playlist: true,
            exchange_id: None,
        }
    }
}

/// Database map, including its file data.
pub struct MapEvidence {
    pub metadata: Map,
    pub data: Vec<u8>,
}

impl std::fmt::Debug for MapEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.metadata.fmt(f)
    }
}

/// A player's preference towards a map.
#[derive(Debug)]
pub struct Preference {
    pub player_login: String,
    pub map_uid: String,
    pub value: PreferenceValue,
}

#[derive(Debug, Clone, Copy, ToSql, FromSql, Deserialize_repr, Serialize_repr)]
#[postgres(name = "pref")]
#[repr(u8)]
pub enum PreferenceValue {
    // None = 0,
    Pick = 1,
    Veto = 2,
    Remove = 3,
}

/// Record data used when inserting into the database.
#[derive(Debug)]
pub struct RecordEvidence {
    pub player_login: String,
    pub map_uid: String,
    pub millis: i32,
    pub timestamp: NaiveDateTime,
    pub sectors: Vec<RecordSector>,
}

/// Detailed checkpoint data recorded at the end of a sector.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordSector {
    /// First sector has index 0, and so on.
    pub index: i32,

    /// Total millis at time of crossing the checkpoint at this index.
    pub cp_millis: i32,

    /// Speed at time of crossing the checkpoint at this index.
    pub cp_speed: f32,
}

/// Detailed record data, that is only missing speed & distance
/// for each checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// The UID of the map this record was set on.
    pub map_uid: String,

    /// The player's map rank, which is the rank of this record
    /// in the ranking of all records on this map.
    pub map_rank: i64,

    /// The login of the player that has set this record.
    pub player_login: String,

    /// The formatted nick name of the player that has set this record.
    pub player_nick_name: GameString,

    /// The duration of this record run in milliseconds.
    pub millis: i32,

    /// The moment this record was set.
    pub timestamp: NaiveDateTime,

    /// Checkpoint data.
    pub sectors: Vec<RecordSector>,
}

/// A rank of a player's record on a specific map.
#[derive(Debug)]
pub struct MapRank {
    pub map_uid: String,
    pub player_login: String,
    pub player_nick_name: GameString,

    /// The player's map rank; if a player has set the best record on a map,
    /// their `pos` is `1`, and so on.
    pub pos: i64,

    /// The maximum map rank; or the number of players that have set a
    /// record on this map.
    pub max_pos: i64,

    /// `True` if the map is in the current playlist.
    pub in_playlist: bool,
}
