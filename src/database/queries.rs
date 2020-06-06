use anyhow::Result;

use async_trait::async_trait;

use crate::database::structs::*;
use crate::server::PlayerInfo;

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
    ) -> Result<Option<RecordDetailed>> {
        Ok(self
            .player_records(map_uid, vec![player_login])
            .await?
            .into_iter()
            .next())
    }
    /// Return the personal best for each of the specified players on the specified map,
    /// if they have one.
    async fn player_records(
        &self,
        map_uid: &str,
        player_logins: Vec<&str>,
    ) -> Result<Vec<RecordDetailed>>;

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

    /// Delete a player, their preferences, and their records.
    /// The data is lost forever.
    async fn delete_player(&self, player_login: &str) -> Result<Option<Player>>;

    /// Delete a map, its preferences, and its records.
    /// The data is lost forever.
    async fn delete_map(&self, map_uid: &str) -> Result<Option<Map>>;

    /// Delete ghost replays of records on every map, that have a rank worse
    /// than the specified one. For example, if `max_rank` is three, then every
    /// record at rank four and higher will have their ghost replay deleted.
    /// The replay data is lost forever, but we usually only care about replays
    /// of the best records anyway.
    ///
    /// # Panics
    /// This function panics if `max_rank` is smaller than one, since we must
    /// never delete every ghost replay.
    async fn delete_old_ghosts(&self, max_rank: i64) -> Result<u64>;
}

#[cfg(test)]
pub mod test {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::database::Database;
    use crate::server::GameString;

    use super::*;
    use chrono::Utc;

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

        pub fn push_player(&mut self, login: &str, nick_name: &str) {
            self.players.push(Player {
                login: login.to_string(),
                nick_name: GameString::from(nick_name.to_string()),
            });
        }

        pub fn push_map(&mut self, uid: &str, in_playlist: bool) {
            self.maps.push(MapEvidence {
                metadata: Map {
                    uid: uid.to_string(),
                    file_name: "".to_string(),
                    name: GameString::from("".to_string()),
                    author_login: "".to_string(),
                    added_since: Utc::now().naive_utc(),
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
                sectors: vec![],
                validation: vec![],
                ghost: None,
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

        async fn upsert_player(&self, _player: &PlayerInfo) -> Result<()> {
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

        async fn nb_records(&self, _map_uid: &str) -> Result<i64> {
            unimplemented!()
        }

        async fn top_record(&self, _map_uid: &str) -> Result<Option<RecordDetailed>> {
            unimplemented!()
        }

        async fn top_records(&self, _map_uid: &str, _limit: i64) -> Result<Vec<Record>> {
            unimplemented!()
        }

        async fn player_records(
            &self,
            _map_uid: &str,
            _player_logins: Vec<&str>,
        ) -> Result<Vec<RecordDetailed>> {
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

        async fn players_without_map_record(&self, _map_uid: &str) -> Result<Vec<String>> {
            unimplemented!()
        }

        async fn record_preview(&self, _record: &RecordEvidence) -> Result<i32> {
            unimplemented!()
        }

        async fn upsert_record(&self, _rec: &RecordEvidence) -> Result<()> {
            unimplemented!()
        }

        async fn player_preferences(&self, _player_login: &str) -> Result<Vec<Preference>> {
            unimplemented!()
        }

        async fn map_preferences(&self, _map_uid: &str) -> Result<Vec<Preference>> {
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
                        let player_nick_name =
                            self.expect_player(&rec.player_login).nick_name.clone();
                        let in_playlist = self.expect_map(&rec.map_uid).in_playlist;
                        MapRank {
                            map_uid: map_uid.to_string(),
                            player_login: rec.player_login.clone(),
                            player_nick_name,
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

        async fn delete_old_ghosts(&self, _max_rank: i64) -> Result<u64> {
            unimplemented!()
        }
    }
}
