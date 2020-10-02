use chrono::NaiveDateTime;
use postgres_types::{FromSql, ToSql};
use serde_repr::Serialize_repr;

use crate::server::DisplayString;

/// Database player that has joined the server at least once.
#[derive(Debug, PartialEq)]
pub struct Player {
    /// Player login.
    pub login: String,

    /// Formatted display name.
    pub display_name: DisplayString,
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

/// A player's preference towards a map.
#[derive(Debug)]
pub struct Preference {
    pub player_login: String,
    pub map_uid: String,
    pub value: PreferenceValue,
}

#[derive(Debug, Clone, Copy, ToSql, FromSql, Serialize_repr)]
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
    pub nb_laps: i32,
    pub millis: i32,
    pub timestamp: NaiveDateTime,
}

/// Detailed record data, that is only missing speed & distance
/// for each checkpoint.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// The UID of the map this record was set on.
    pub map_uid: String,

    /// The login of the player that has set this record.
    pub player_login: String,

    /// The number of laps for this record.
    ///
    /// Use `0` if the map is not multi-lap, or to count flying lap records.
    pub nb_laps: i32,

    /// The player's map rank, which is the rank of this record
    /// in the ranking of all records with the same lap count on this map.
    pub map_rank: i64,

    /// The formatted display name of the player that has set this record.
    pub player_display_name: DisplayString,

    /// The duration of this record run in milliseconds.
    pub millis: i32,

    /// The moment this record was set.
    pub timestamp: NaiveDateTime,
}

/// A rank of a player's record on a specific map.
///
/// On multi-lap maps, this is the
#[derive(Debug)]
pub struct MapRank {
    pub map_uid: String,
    pub player_login: String,
    pub player_display_name: DisplayString,

    /// The player's map rank; if a player has set the best record on a map,
    /// their `pos` is `1`, and so on.
    pub pos: i64,

    /// The maximum map rank; or the number of players that have set a
    /// record on this map.
    pub max_pos: i64,
}
