use std::time::SystemTime;

use postgres_types::{FromSql, ToSql};
use serde_repr::{Deserialize_repr, Serialize_repr};

use gbx::MapInfo;

/// Database player that has joined the server at least once.
#[derive(Debug)]
pub struct Player {
    /// Player login.
    pub login: String,

    /// Formatted nick name.
    pub nick_name: String,
}

/// Database map.
#[derive(Clone, Debug)]
pub struct Map {
    /// Unique identifier.
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,

    /// The formatted map name.
    pub name: String,

    /// The map author's login.
    pub author_login: String,

    /// The moment this map was added to the database.
    pub added_since: SystemTime,

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
            added_since: SystemTime::now(),
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

#[derive(
    ToSql, FromSql, Deserialize_repr, Serialize_repr, Debug, PartialEq, Eq, Clone, Hash, Copy,
)]
#[repr(u8)]
#[postgres(name = "steward.Pref")]
pub enum PreferenceValue {
    // None = 0,
    // AutoPick = 1,
    Pick = 2,
    Veto = 3,
    Remove = 4,
}

/// Complete record variant, that includes replay data as "evidence".
/// Should only be used to store data, or when exporting it.
pub struct RecordEvidence {
    pub player_login: String,
    pub map_uid: String,
    pub millis: i32,
    pub timestamp: SystemTime,
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

    /// Total driven distance at time of crossing the checkpoint at this index.
    pub cp_distance: f32,
}

/// Detailed record data, that is only missing speed & distance
/// for each checkpoint.
#[derive(Clone, Debug)]
pub struct RecordDetailed {
    /// The player's map rank, which is the rank of this record
    /// in the ranking of all records on this map.
    pub map_rank: i64,

    /// The login of the player that has set this record.
    pub player_login: String,

    /// The formatted nick name of the player that has set this record.
    pub player_nick_name: String,

    /// The duration of this record run in milliseconds.
    pub millis: i32,

    /// The moment this record was set.
    pub timestamp: SystemTime,

    /// The milliseconds at the time of passing each checkpoint -
    /// the finish line being the last.
    pub cp_millis: Vec<i32>,
}

impl RecordDetailed {
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
    pub player_nick_name: String,
    pub millis: i32,
    pub timestamp: SystemTime,
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
    pub player_nick_name: String,

    /// The player's map rank; if a player has set the best record on a map,
    /// their `pos` is `1`, and so on.
    pub pos: i64,

    /// The maximum map rank; or the number of players that have set a
    /// record on this map.
    pub max_pos: i64,
}
