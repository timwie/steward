use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use chrono::NaiveDateTime;
use chrono::Utc;

use crate::database::timeattack::*;
use crate::database::*;
use crate::server::DisplayString;
use crate::server::PlayerInfo;

pub type Result<T> = anyhow::Result<T>;

#[derive(Default)]
pub struct DatabaseClient {
    pub mock: MockDatabase,
}

#[derive(Default)]
pub struct MockDatabase {
    pub maps: Vec<Map>,
    pub players: Vec<Player>,
    pub records: Vec<RecordEvidence>,
}

impl Clone for DatabaseClient {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}

impl DatabaseClient {
    pub async fn migrate(&self) -> Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl PlayerQueries for DatabaseClient {
    async fn player(&self, _login: &str) -> Result<Option<Player>> {
        unimplemented!()
    }

    async fn players(&self, _logins: Vec<&str>) -> Result<Vec<Player>> {
        unimplemented!()
    }

    async fn upsert_player(&self, _player: &PlayerInfo) -> Result<()> {
        unimplemented!()
    }

    async fn delete_player(&self, _player_login: &str) -> Result<Option<Player>> {
        unimplemented!()
    }
}

#[async_trait]
impl MapQueries for DatabaseClient {
    async fn map_file(&self, _uid: &str) -> Result<Option<Vec<u8>>> {
        unimplemented!()
    }

    async fn maps(&self, _map_uids: Vec<&str>) -> Result<Vec<Map>> {
        unimplemented!()
    }

    async fn map(&self, _map_uid: &str) -> Result<Option<Map>> {
        unimplemented!()
    }

    async fn upsert_map(&self, _metadata: &Map, _data: Vec<u8>) -> Result<()> {
        unimplemented!()
    }

    async fn delete_map(&self, _map_uid: &str) -> Result<Option<Map>> {
        unimplemented!()
    }
}

#[async_trait]
impl RecordQueries for DatabaseClient {
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

    async fn top_record(&self, map_uid: &str, nb_laps: i32) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(1))
            .await?
            .into_iter()
            .next())
    }

    async fn top_records(&self, map_uid: &str, limit: i64, nb_laps: i32) -> Result<Vec<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(limit))
            .await?)
    }

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

    async fn nb_players_with_record(&self) -> Result<i64> {
        let logins: HashSet<&str> = self
            .mock
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
}

#[async_trait]
impl TimeAttackQueries for DatabaseClient {
    async fn add_history(
        &self,
        _player_login: &str,
        _map_uid: &str,
        _last_played: &NaiveDateTime,
    ) -> Result<()> {
        unimplemented!()
    }

    async fn history(&self, _player_login: &str, _map_uids: Vec<&str>) -> Result<Vec<History>> {
        unimplemented!()
    }

    async fn player_preferences(&self, _player_login: &str) -> Result<Vec<Preference>> {
        unimplemented!()
    }

    async fn count_map_preferences(&self, _map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>> {
        unimplemented!()
    }

    async fn upsert_preference(&self, _pref: &Preference) -> Result<()> {
        unimplemented!()
    }

    async fn map_rankings(&self, map_uids: Vec<&str>) -> Result<Vec<MapRank>> {
        let db = &self.mock;
        let mut grp_by_map = HashMap::<&str, Vec<&RecordEvidence>>::new();
        for rec in db.records.iter() {
            if !map_uids.contains(&rec.map_uid.as_str()) {
                continue;
            }
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
                        db.expect_player(&rec.player_login).display_name.clone();
                    MapRank {
                        map_uid: map_uid.to_string(),
                        player_login: rec.player_login.clone(),
                        player_display_name,
                        pos: idx as i64 + 1,
                        max_pos,
                    }
                })
            })
            .collect())
    }
}

impl DatabaseClient {
    pub fn push_player(&mut self, login: &str, display_name: &str) {
        let db = &mut self.mock;
        db.players.push(Player {
            login: login.to_string(),
            display_name: DisplayString::from(display_name.to_string()),
        });
    }

    pub fn push_map(&mut self, uid: &str) {
        let db = &mut self.mock;
        db.maps.push(Map {
            uid: uid.to_string(),
            file_name: "".to_string(),
            name: DisplayString::from("".to_string()),
            author_login: "".to_string(),
            author_display_name: DisplayString::from("".to_string()),
            added_since: Utc::now().naive_utc(),
            author_millis: 0,
            exchange_id: None,
        });
    }

    pub fn push_record(&mut self, login: &str, uid: &str, millis: i32) {
        let db = &mut self.mock;
        db.records.push(RecordEvidence {
            player_login: login.to_string(),
            map_uid: uid.to_string(),
            millis,
            timestamp: Utc::now().naive_utc(),
            nb_laps: 0,
        });
    }
}

impl MockDatabase {
    pub fn expect_player(&self, login: &str) -> &Player {
        self.players
            .iter()
            .find(|p| p.login == login)
            .expect("player login not in mock database")
    }

    pub fn expect_map(&self, uid: &str) -> &Map {
        &self
            .maps
            .iter()
            .find(|m| m.uid == uid)
            .expect("map uid not in mock database")
    }
}
