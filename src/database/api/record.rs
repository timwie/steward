use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::database::Result;
use crate::server::DisplayString;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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

#[async_trait]
pub trait RecordQueries {
    /// Return the number of players that have set a record on the specified map,
    /// with the specified lap count.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to count flying lap records.
    async fn nb_records(&self, map_uid: &str, nb_laps: i32) -> Result<i64>;

    /// Return records on the specified maps, set by the specified players, with the specified
    /// amount of laps.
    ///
    /// # Arguments
    /// `map_uids` - A list of map UIDs to return records for. Use an empty list to select
    ///              records for all maps.
    /// `player_logins` - A list of player logins to return records for. Use an empty list to
    ///                   select records set by any player.
    /// `nb_laps` - The number of required laps. Use `0` if the map is not multi-lap,
    ///             or to get flying lap records.
    /// `limit_per_map` - The maximum number of records returned for each specified map.
    async fn records(
        &self,
        map_uids: Vec<&str>,
        player_logins: Vec<&str>,
        nb_laps: i32,
        limit_per_map: Option<i64>,
    ) -> Result<Vec<Record>>;

    /// Return the top record set by any player on the specified map,
    /// with the specified lap count, or `None` if no player has completed such a
    /// run on that map.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get the top flying lap records.
    async fn top_record(&self, map_uid: &str, nb_laps: i32) -> Result<Option<Record>>;

    /// Return limited number of top records on the specified map,
    /// with the specified lap count, sorted from best to worse.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get flying lap records.
    async fn top_records(&self, map_uid: &str, limit: i64, nb_laps: i32) -> Result<Vec<Record>>;

    /// Return the personal best of the specified player on the specified map,
    /// with the specified lap count, or `None` if the player has not completed such a
    /// run on that map.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get the player's flying lap PB.
    async fn player_record(
        &self,
        map_uid: &str,
        player_login: &str,
        nb_laps: i32,
    ) -> Result<Option<Record>>;

    /// Return the number of players that have set a record on at least one map.
    async fn nb_players_with_record(&self) -> Result<i64>;

    /// List all map UIDs that the specified player has not completed a run on.
    async fn maps_without_player_record(&self, player_login: &str) -> Result<Vec<String>>;

    /// Without inserting the given record, return the map rank it would achieve,
    /// if it were inserted.
    async fn record_preview(&self, record: &RecordEvidence) -> Result<i64>;

    /// Updates the player's personal best on a map.
    ///
    /// # Note
    /// If a previous record exists for that player, this function does not
    /// check if the given record is actually better than the one in the database.
    async fn upsert_record(&self, rec: &RecordEvidence) -> Result<()>;
}
