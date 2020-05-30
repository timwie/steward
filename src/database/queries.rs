use anyhow::Result;

use async_trait::async_trait;

use crate::database::structs::*;
use crate::ingame::PlayerInfo;

#[async_trait]
pub trait Queries: Send + Sync {
    /// Check for pending database migrations and execute them.
    async fn migrate(&self) -> Result<()>;

    /// Return the specified player, or `None` if no such player exists in the database.
    async fn player(&self, login: &str) -> Result<Option<Player>>;

    /// Insert a player into the database.
    /// Update their nick name if the player already exists.
    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()>;

    /// List all maps, including their file data.
    async fn map_files(&self) -> Result<Vec<MapEvidence>>;

    /// List all maps.
    async fn maps(&self) -> Result<Vec<Map>>;

    /// List all maps in the playlist.
    async fn playlist(&self) -> Result<Vec<Map>>;

    /// Return the specified map, or `None` if no such map exists in the database.
    async fn map(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Insert the map and add it to the playlist if it does not already
    /// exist in the database. Update its file path & exchange ID otherwise.
    async fn upsert_map(&self, map: &MapEvidence) -> Result<()>;

    /// Add the specified map to the playlist, and return it,
    /// or `None` if there is no map with that UID.
    async fn playlist_add(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Remove the specified map from the playlist, and return it,
    /// or `None` if there is no map with that UID.
    async fn playlist_remove(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Return the number of players that have set a record on the specified map.
    async fn nb_records(&self, map_uid: &str) -> Result<i64>;

    /// Return the top record set by any player on the specified map,
    /// or `None` if no player has completed a run on that map.
    async fn top_record(&self, map_uid: &str) -> Result<Option<RecordDetailed>>;

    /// Return limited number of top records on the specified map,
    /// sorted from best to worse.
    async fn top_records(&self, map_uid: &str, limit: i64) -> Result<Vec<Record>>;

    /// Return the personal best of the specified player on the specified map,
    /// or `None` if the player has not completed a run on that map.
    async fn player_record(
        &self,
        map_uid: &str,
        player_login: &str,
    ) -> Result<Option<RecordDetailed>>;

    /// Return the number of players that have set a record on at least one map.
    async fn nb_players_with_record(&self) -> Result<i64>;

    /// List all map UIDs that the specified player has not completed a run on.
    async fn maps_without_player_record(&self, player_login: &str) -> Result<Vec<String>>;

    /// List UIDs of all players that have *not* completed a run on the specified map.
    async fn players_without_map_record(&self, map_uid: &str) -> Result<Vec<String>>;

    /// Without inserting the given record, return the map rank it would achieve,
    /// if it were inserted.
    async fn record_preview(&self, record: &RecordEvidence) -> Result<i32>;

    /// Updates the player's personal best on a map.
    ///
    /// # Note
    /// If a previous record exists for that player, this function does not
    /// check if the given record is actually better than the one in the database.
    async fn upsert_record(&self, rec: &RecordEvidence) -> Result<()>;

    /// List all preferences that the specified player has set.
    async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>>;

    /// List preferences set by any player, for the specified map.
    async fn map_preferences(&self, map_uid: &str) -> Result<Vec<Preference>>;

    /// Count the number of times each preference was set by any player, for the specified map.
    async fn count_map_preferences(&self, map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>>;

    /// Insert a player's map preference, overwriting any previous preference.
    async fn upsert_preference(&self, pref: &Preference) -> Result<()>;

    /// Calculate the map rank of *every* player.
    ///
    /// # Note
    /// The length of this collection is equal to the total number of records
    /// stored in the database. This function should only be used when calculating
    /// the server ranking.
    async fn map_rankings(&self) -> Result<Vec<MapRank>>;
}
