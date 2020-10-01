use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::database::structs::*;
use crate::server::PlayerInfo;

#[async_trait]
pub trait Queries: Send + Sync {
    /// Check for pending database migrations and execute them.
    async fn migrate(&self) -> Result<()>;

    /// Return the specified player, or `None` if no such player exists in the database.
    async fn player(&self, login: &str) -> Result<Option<Player>>;

    /// Return players for every input login that exists in the database.
    async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>>;

    /// Insert a player into the database.
    /// Update their display name if the player already exists.
    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()>;

    /// Update a player's history, setting *now* as the time they most recently
    /// played the specified map.
    async fn add_history(
        &self,
        player_login: &str,
        map_uid: &str,
        last_played: &NaiveDateTime,
    ) -> Result<()>;

    /// Returns the player's history for each map currently in the playlist.
    async fn history(&self, player_login: &str) -> Result<Vec<History>>;

    /// List all maps, including their file data.
    async fn map_files(&self) -> Result<Vec<MapEvidence>>;

    /// List all maps.
    async fn maps(&self) -> Result<Vec<Map>>;

    /// List all maps in the playlist.
    async fn playlist(&self) -> Result<Vec<Map>>;

    /// Return the specified map, or `None` if no such map exists in the database.
    async fn map(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Insert a map into the database.
    ///
    /// If the given map already exists in the database, update
    ///  - its file
    ///  - its file path
    ///  - whether it is in the playlist
    ///  - its exchange ID.
    async fn upsert_map(&self, map: &MapEvidence) -> Result<()>;

    /// Add the specified map to the playlist, and return it,
    /// or `None` if there is no map with that UID.
    async fn playlist_add(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Remove the specified map from the playlist, and return it,
    /// or `None` if there is no map with that UID.
    async fn playlist_remove(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Return the number of players that have set a record on the specified map,
    /// with the specified lap count.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to count flying lap records.
    async fn nb_records(&self, map_uid: &str, nb_laps: i32) -> Result<i64>;

    /// Return the top record set by any player on the specified map,
    /// with the specified lap count, or `None` if no player has completed such a
    /// run on that map.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get the top flying lap records.
    async fn top_record(&self, map_uid: &str, nb_laps: i32) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(1))
            .await?
            .into_iter()
            .next())
    }

    /// Return limited number of top records on the specified map,
    /// with the specified lap count, sorted from best to worse.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get flying lap records.
    async fn top_records(&self, map_uid: &str, limit: i64, nb_laps: i32) -> Result<Vec<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(limit))
            .await?)
    }

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
    ) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![player_login], nb_laps, None)
            .await?
            .into_iter()
            .next())
    }

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

    /// List all preferences that the specified player has set.
    async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>>;

    /// Count the number of times each preference was set by any player, for the specified map.
    async fn count_map_preferences(&self, map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>>;

    /// Insert a player's map preference, overwriting any previous preference.
    async fn upsert_preference(&self, pref: &Preference) -> Result<()>;

    /// Calculate the map rank of *every* player.
    ///
    /// For multi-lap maps, the best map rank will have the best flying lap.
    ///
    /// # Note
    /// The length of this collection is equal to the total number of `nb_laps == 0` records
    /// stored in the database. This function should only be used when calculating
    /// the server ranking.
    async fn map_rankings(&self) -> Result<Vec<MapRank>>;

    /// Delete a player, their preferences, and their records.
    /// The data is lost forever.
    async fn delete_player(&self, player_login: &str) -> Result<Option<Player>>;

    /// Delete a map, its preferences, and its records.
    /// The data is lost forever.
    async fn delete_map(&self, map_uid: &str) -> Result<Option<Map>>;
}

#[cfg(test)]
pub mod test {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::Utc;

    use crate::database::Database;
    use crate::server::DisplayString;

    use super::*;

    pub struct MockDatabase {
        pub maps: Vec<MapEvidence>,
        pub players: Vec<Player>,
        pub records: Vec<RecordEvidence>,
    }

    impl MockDatabase {
        pub fn new() -> Self {
            MockDatabase {
                maps: vec![],
                players: vec![],
                records: vec![],
            }
        }

        pub fn into_arc(self) -> Arc<dyn Database> {
            Arc::new(self) as Arc<dyn Database>
        }

        pub fn push_player(&mut self, login: &str, display_name: &str) {
            self.players.push(Player {
                login: login.to_string(),
                display_name: DisplayString::from(display_name.to_string()),
            });
        }

        pub fn push_map(&mut self, uid: &str, in_playlist: bool) {
            self.maps.push(MapEvidence {
                metadata: Map {
                    uid: uid.to_string(),
                    file_name: "".to_string(),
                    name: DisplayString::from("".to_string()),
                    author_login: "".to_string(),
                    author_display_name: DisplayString::from("".to_string()),
                    added_since: Utc::now().naive_utc(),
                    author_millis: 0,
                    in_playlist,
                    exchange_id: None,
                },
                data: vec![],
            });
        }

        pub fn push_record(&mut self, login: &str, uid: &str, millis: i32) {
            self.records.push(RecordEvidence {
                player_login: login.to_string(),
                map_uid: uid.to_string(),
                millis,
                timestamp: Utc::now().naive_utc(),
                nb_laps: 0,
            });
        }

        fn expect_player(&self, login: &str) -> &Player {
            self.players
                .iter()
                .find(|p| p.login == login)
                .expect("player login not in mock database")
        }

        fn expect_map(&self, uid: &str) -> &Map {
            &self
                .maps
                .iter()
                .find(|m| m.metadata.uid == uid)
                .expect("map uid not in mock database")
                .metadata
        }
    }

    #[async_trait]
    impl Queries for MockDatabase {
        async fn migrate(&self) -> Result<()> {
            unimplemented!()
        }

        async fn player(&self, _login: &str) -> Result<Option<Player>> {
            unimplemented!()
        }

        async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>> {
            unimplemented!()
        }

        async fn upsert_player(&self, _player: &PlayerInfo) -> Result<()> {
            unimplemented!()
        }

        async fn add_history(
            &self,
            _player_login: &str,
            _map_uid: &str,
            _last_played: &NaiveDateTime,
        ) -> Result<()> {
            unimplemented!()
        }

        async fn history(&self, _player_login: &str) -> Result<Vec<History>> {
            unimplemented!()
        }

        async fn map_files(&self) -> Result<Vec<MapEvidence>> {
            unimplemented!()
        }

        async fn maps(&self) -> Result<Vec<Map>> {
            unimplemented!()
        }

        async fn playlist(&self) -> Result<Vec<Map>> {
            Ok(self
                .maps
                .iter()
                .filter(|ev| ev.metadata.in_playlist)
                .map(|ev| ev.metadata.clone())
                .collect())
        }

        async fn map(&self, _map_uid: &str) -> Result<Option<Map>> {
            unimplemented!()
        }

        async fn upsert_map(&self, _map: &MapEvidence) -> Result<()> {
            unimplemented!()
        }

        async fn playlist_add(&self, _map_uid: &str) -> Result<Option<Map>> {
            unimplemented!()
        }

        async fn playlist_remove(&self, _map_uid: &str) -> Result<Option<Map>> {
            unimplemented!()
        }

        async fn nb_records(&self, _map_uid: &str, _nb_laps: i32) -> Result<i64> {
            unimplemented!()
        }

        async fn records(
            &self,
            _map_uids: Vec<&str>,
            _player_logins: Vec<&str>,
            _nb_laps: i32,
            _limit_per_map: Option<i64>,
        ) -> Result<Vec<Record>> {
            unimplemented!()
        }

        async fn nb_players_with_record(&self) -> Result<i64> {
            let logins: HashSet<&str> = self
                .records
                .iter()
                .map(|rec| rec.player_login.as_str())
                .collect();
            Ok(logins.len() as i64)
        }

        async fn maps_without_player_record(&self, _player_login: &str) -> Result<Vec<String>> {
            unimplemented!()
        }

        async fn record_preview(&self, _record: &RecordEvidence) -> Result<i64> {
            unimplemented!()
        }

        async fn upsert_record(&self, _rec: &RecordEvidence) -> Result<()> {
            unimplemented!()
        }

        async fn player_preferences(&self, _player_login: &str) -> Result<Vec<Preference>> {
            unimplemented!()
        }

        async fn count_map_preferences(
            &self,
            _map_uid: &str,
        ) -> Result<Vec<(PreferenceValue, i64)>> {
            unimplemented!()
        }

        async fn upsert_preference(&self, _pref: &Preference) -> Result<()> {
            unimplemented!()
        }

        async fn map_rankings(&self) -> Result<Vec<MapRank>> {
            let mut grp_by_map = HashMap::<&str, Vec<&RecordEvidence>>::new();
            for rec in self.records.iter() {
                grp_by_map.entry(&rec.map_uid).or_insert(vec![]).push(&rec);
            }
            for map_recs in grp_by_map.values_mut() {
                map_recs.sort_by_key(|rec| rec.millis);
            }
            Ok(grp_by_map
                .into_iter()
                .flat_map(|(map_uid, map_recs)| {
                    let max_pos = map_recs.len() as i64;
                    map_recs.into_iter().enumerate().map(move |(idx, rec)| {
                        let player_display_name =
                            self.expect_player(&rec.player_login).display_name.clone();
                        let in_playlist = self.expect_map(&rec.map_uid).in_playlist;
                        MapRank {
                            map_uid: map_uid.to_string(),
                            player_login: rec.player_login.clone(),
                            player_display_name,
                            pos: idx as i64 + 1,
                            max_pos,
                            in_playlist,
                        }
                    })
                })
                .collect())
        }

        async fn delete_player(&self, _player_login: &str) -> Result<Option<Player>> {
            unimplemented!()
        }

        async fn delete_map(&self, _map_uid: &str) -> Result<Option<Map>> {
            unimplemented!()
        }
    }
}
