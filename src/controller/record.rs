use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use futures::future::join_all;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::config::MAX_DISPLAYED_MAP_RANKS;
use crate::controller::{LivePlayers, LivePlaylist};
use crate::database::{Database, Map, Record, RecordEvidence, RecordSector};
use crate::event::{PbDiff, PlayerDiff, PlayerTransition};
use crate::server::PlayerInfo;
use crate::server::{CheckpointEvent, Server};

/// Shared component that allows to look up records
/// of the current map.
#[async_trait]
pub trait LiveRecords: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, RecordsState>;
}

pub struct RecordsState {
    /// The number of players that have set a record on the current map.
    pub nb_records: usize,

    /// The top record on the current map, or `None` if no player
    /// has set a record on the current map.
    pub top_record: Option<Record>,

    /// A number of top records on the current map. The number is determined
    /// by how many records we want to display in-game. The vector is sorted
    /// from better to worse.
    pub top_records: Vec<Record>,

    /// Maps player UID to their personal best on the current map.
    pbs: HashMap<i32, Record>,

    /// Maps player UID to the recorded sector data in their current run.
    run_sectors: HashMap<i32, Vec<RecordSector>>,
}

impl RecordsState {
    fn init() -> Self {
        RecordsState {
            nb_records: 0,
            top_record: None,
            top_records: vec![],
            pbs: HashMap::new(),
            run_sectors: HashMap::new(),
        }
    }

    /// The personal best of the specified player, on the current map, or
    /// `None` if they have not set a record yet.
    pub fn pb(&self, player_uid: i32) -> Option<&Record> {
        self.pbs.get(&player_uid)
    }

    /// Iterate through the records of only connected players.
    pub fn playing_pbs(&self) -> impl Iterator<Item = &Record> {
        self.pbs.values()
    }

    /// Without inserting the given record, return the map rank it would achieve,
    /// if it were inserted. Returns `None` if the record would not enter the
    /// top records that are cached in this state.
    fn pos_preview(&self, rec: &RecordEvidence) -> Option<usize> {
        if self.top_records.is_empty() {
            Some(1)
        } else {
            self.top_records
                .iter()
                .position(|other| rec.millis < other.millis)
                .map(|idx| idx + 1)
        }
    }

    fn upsert_record(&mut self, player_uid: i32, record: &Record) {
        let is_first_record = !self.pbs.contains_key(&player_uid);
        if is_first_record {
            self.nb_records += 1;
        }

        let is_new_pb = is_first_record
            || self
                .pbs
                .get(&player_uid)
                .filter(|pb| pb.millis <= record.millis)
                .is_none();
        if is_new_pb {
            self.pbs.insert(player_uid, record.clone());
        }

        // Remove a previous top n record set by this player.
        let prev_ranking_idx = self
            .top_records
            .iter()
            .position(|rec| rec.player_login == record.player_login);
        if let Some(idx) = prev_ranking_idx {
            self.top_records.remove(idx);
        }

        // Update the cached top records if the record is good enough.
        let record_ranking_idx = if self.top_records.is_empty() {
            Some(0)
        } else {
            self.top_records
                .iter()
                .position(|other| record.millis < other.millis)
        };

        if let Some(idx) = record_ranking_idx {
            self.top_records.insert(idx, record.clone());
            self.top_records.truncate(MAX_DISPLAYED_MAP_RANKS);

            if idx == 0 {
                self.top_record = Some(record.clone());
            }
        }
    }
}

#[derive(Clone)]
pub struct RecordController {
    server: Arc<dyn Server>,
    db: Arc<dyn Database>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_players: Arc<dyn LivePlayers>,
    state: Arc<RwLock<RecordsState>>,
}

impl RecordController {
    pub async fn init(
        server: &Arc<dyn Server>,
        db: &Arc<dyn Database>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_players: &Arc<dyn LivePlayers>,
    ) -> Self {
        let controller = RecordController {
            server: server.clone(),
            db: db.clone(),
            live_playlist: live_playlist.clone(),
            live_players: live_players.clone(),
            state: Arc::new(RwLock::new(RecordsState::init())),
        };

        if let Some(map) = live_playlist.current_map().await {
            controller.load_for_map(&map).await;

            join_all(
                live_players
                    .lock()
                    .await
                    .info_all()
                    .iter()
                    .map(|info| controller.load_for_player(&map.uid, &info)),
            )
            .await;
        }

        controller
    }

    /// Load a player's personal best when they join, or unload it when they leave.
    pub async fn update_for_player(&self, diff: &PlayerDiff) {
        use PlayerTransition::*;

        let map_uid = match self.live_playlist.current_map_uid().await {
            Some(uid) => uid,
            None => return,
        };

        match diff.transition {
            AddPlayer | AddSpectator | AddPureSpectator => {
                self.load_for_player(&map_uid, &diff.info).await;
            }
            RemovePlayer | RemoveSpectator | RemovePureSpectator => {
                let mut records_state = self.state.write().await;
                records_state.pbs.remove(&diff.info.uid);
                records_state.run_sectors.remove(&diff.info.uid);
            }
            _ => {}
        }
    }

    async fn load_for_player(&self, map_uid: &str, info: &PlayerInfo) {
        let pb = self
            .db
            .player_record(&map_uid, &info.login)
            .await
            .expect("failed to load player PB");
        if let Some(pb) = pb {
            let mut records_state = self.state.write().await;
            records_state.pbs.insert(info.uid, pb);
        }
    }

    /// Load a map's top records, and records of connected players.
    pub async fn load_for_map(&self, loaded_map: &Map) {
        let nb_records = self
            .db
            .nb_records(&loaded_map.uid)
            .await
            .expect("failed to load number of map records") as usize;

        let top1 = self
            .db
            .top_record(&loaded_map.uid)
            .await
            .expect("failed to load map's top1 record");

        let top_records = self
            .db
            .top_records(&loaded_map.uid, MAX_DISPLAYED_MAP_RANKS as i64)
            .await
            .expect("failed to load map records");

        let mut records_state = self.state.write().await;
        records_state.run_sectors.clear();
        records_state.top_record = top1;
        records_state.top_records = top_records;
        records_state.nb_records = nb_records;
        records_state.pbs.clear();

        let players_state = self.live_players.lock().await;
        let all_logins = players_state
            .info_all()
            .iter()
            .map(|info| info.login.as_str())
            .collect();
        let pbs = self
            .db
            .records(vec![&loaded_map.uid], all_logins, None)
            .await
            .expect("failed to load player PBs");

        for pb in pbs {
            if let Some(uid) = players_state.uid(&pb.player_login) {
                records_state.pbs.insert(*uid, pb);
            }
        }
    }

    /// Add new sector data for a player's current run.
    pub async fn update_run(&self, ev: &CheckpointEvent) {
        if let Some(player_info) = self.live_players.info(&ev.player_login).await {
            let mut records_state = self.state.write().await;
            records_state
                .run_sectors
                .entry(player_info.uid)
                .or_insert_with(Vec::new)
                .push(RecordSector {
                    index: ev.cp_index,
                    cp_millis: ev.race_time_millis,
                    cp_speed: ev.speed.abs(), // driving backwards gives negative speed
                });
        }
    }

    /// Produce a record from the collected sector data at the end of a run.
    /// If that run is the player's new personal best, update the map records.
    ///
    /// The cached personal best for this player will be updated,
    /// and if it is a top n record, that cached list will also be
    /// updated.
    pub async fn end_run(&self, finish_ev: &CheckpointEvent) -> Option<PbDiff> {
        assert!(finish_ev.is_finish);

        let player = match self.live_players.info(&finish_ev.player_login).await {
            Some(player_info) => player_info,
            None => return None,
        };

        let map_uid = match self.live_playlist.current_map_uid().await {
            Some(uid) => uid,
            None => return None,
        };

        let mut records_state = self.state.write().await;

        let prev_pb = records_state.pb(player.uid);
        let prev_pb_pos = prev_pb.map(|rec| rec.map_rank as usize);
        let prev_pb_diff = prev_pb.map(|rec| finish_ev.race_time_millis - rec.millis);

        let is_new_pb = prev_pb_diff.map(|millis| millis < 0).unwrap_or(true);
        if !is_new_pb {
            return Some(PbDiff {
                player_uid: player.uid,
                millis_diff: prev_pb_diff,
                prev_pos: prev_pb_pos,
                new_pos: prev_pb_pos.unwrap(), // no change in position
                new_record: None,
                pos_gained: 0,
            });
        }

        let sectors = records_state
            .run_sectors
            .remove(&player.uid)
            .expect("no sector data");

        let evidence = RecordEvidence {
            player_login: player.login.clone(),
            map_uid,
            millis: finish_ev.race_time_millis,
            timestamp: Utc::now().naive_utc(),
            sectors,
        };

        // We already know the rank of the new record if it is better
        // than at least one of the cached records. Otherwise,
        // we have to look it up in the database.
        let new_pos: usize = match records_state.pos_preview(&evidence) {
            Some(pos) => pos,
            None => self
                .db
                .record_preview(&evidence)
                .await
                .expect("failed to check record rank") as usize,
        };

        // Remember record in the database.
        self.db
            .upsert_record(&evidence)
            .await
            .expect("failed to update player PB");

        let record = Record {
            map_uid: evidence.map_uid,
            map_rank: new_pos as i64,
            player_login: player.login.clone(),
            player_nick_name: player.nick_name.clone(),
            timestamp: evidence.timestamp,
            millis: evidence.millis,
            sectors: evidence.sectors,
        };

        // Remember record in the cache.
        records_state.upsert_record(player.uid, &record);

        let pos_gained = match prev_pb_pos {
            Some(p) => p - new_pos,
            None if records_state.nb_records == 1 => 1,
            None => records_state.nb_records - new_pos,
        };

        Some(PbDiff {
            player_uid: player.uid,
            millis_diff: prev_pb_diff,
            new_pos,
            prev_pos: prev_pb_pos,
            pos_gained,
            new_record: Some(record),
        })
    }

    /// Discard all data stored for a player's current run when they
    /// respawn.
    pub async fn reset_run(&self, player_login: &str) {
        if let Some(player_uid) = self.live_players.uid(player_login).await {
            let mut records_state = self.state.write().await;
            records_state.run_sectors.remove(&player_uid);
        }
    }
}

#[async_trait]
impl LiveRecords for RecordController {
    async fn lock(&self) -> RwLockReadGuard<'_, RecordsState> {
        self.state.read().await
    }
}
