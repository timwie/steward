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

/// Complete record variant, that includes replay data as "evidence".
/// Should only be used to store data, or when exporting it.
pub struct RecordEvidence {
    pub player_login: String,
    pub map_uid: String,
    pub millis: i32,
    pub timestamp: NaiveDateTime,
    pub sectors: Vec<RecordSector>,

    /// Validation replay file data.
    pub validation: Vec<u8>,

    /// Ghost replay file data.
    pub ghost: Option<Vec<u8>>,
}

impl std::fmt::Debug for RecordEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordEvidence")
            .field("player_login", &self.player_login)
            .field("map_uid", &self.map_uid)
            .field("millis", &self.millis)
            .field("timestamp", &self.timestamp)
            .field("sectors", &self.sectors)
            .finish()
    }
}

/// Detailed checkpoint data recorded at the end of a sector.
#[derive(Debug)]
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
pub struct RecordDetailed {
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

    /// The milliseconds at the time of passing each checkpoint -
    /// the finish line being the last.
    pub cp_millis: Vec<i32>,
}

impl RecordDetailed {
    #[allow(dead_code)]
    pub fn sector_millis(&self) -> Vec<usize> {
        let mut sector_times = Vec::with_capacity(self.cp_millis.len());
        let mut offset_millis: usize = 0;
        for millis in self.cp_millis.iter() {
            sector_times.push(*millis as usize - offset_millis);
            offset_millis = *millis as usize;
        }
        sector_times
    }
}

/// Record variant without rank & sector data.
#[derive(Debug)]
pub struct Record {
    pub player_login: String,
    pub player_nick_name: GameString,
    pub millis: i32,
    pub timestamp: NaiveDateTime,
}

impl From<RecordDetailed> for Record {
    fn from(rec: RecordDetailed) -> Self {
        Record {
            player_login: rec.player_login,
            player_nick_name: rec.player_nick_name,
            millis: rec.millis,
            timestamp: rec.timestamp,
        }
    }
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
