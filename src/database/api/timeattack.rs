use async_trait::async_trait;
use chrono::NaiveDateTime;
use postgres_types::{FromSql, ToSql};
use serde_repr::Serialize_repr;

use crate::database::Result;
use crate::server::DisplayString;

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

#[async_trait]
pub trait TimeAttackQueries {
    /// Update a player's history, setting *now* as the time they most recently
    /// played the specified map.
    async fn add_history(
        &self,
        player_login: &str,
        map_uid: &str,
        last_played: &NaiveDateTime,
    ) -> Result<()>;

    /// Returns the specified player's history for every specified map they have played.
    ///
    /// # Arguments
    /// `player_login` - a player's login
    /// `map_uids` - A list of map UIDs to return the history for. Use an empty list to select
    ///              records for all maps.
    async fn history(&self, player_login: &str, map_uids: Vec<&str>) -> Result<Vec<History>>;

    /// List all preferences that the specified player has set.
    async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>>;

    /// Count the number of times each preference was set by any player, for the specified map.
    async fn count_map_preferences(&self, map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>>;

    /// Insert a player's map preference, overwriting any previous preference.
    async fn upsert_preference(&self, pref: &Preference) -> Result<()>;

    /// Calculate the map rank of *every* player, for each of the specified maps.
    ///
    /// For multi-lap maps, the best map rank will have the best flying lap.
    ///
    /// # Note
    /// The length of this collection is equal to the total number of `nb_laps == 0` records
    /// stored in the database. This function should only be used when calculating
    /// the server ranking.
    async fn map_rankings(&self, map_uids: Vec<&str>) -> Result<Vec<MapRank>>;
}
