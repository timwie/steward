use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::config::{MAX_DISPLAYED_MAP_RANKS, MAX_GHOST_REPLAY_RANK};
use crate::controller::{LivePlayers, LivePlaylist};
use crate::database::{Database, Map, Record, RecordDetailed, RecordEvidence, RecordSector};
use crate::event::{PbDiff, PlayerDiff};
use crate::server::{CheckpointEvent, Server};

/// Shared component that allows to look up records
/// of the current map.
#[async_trait]
pub trait LiveRecords: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, RecordState>;

    /// The number of players that have set a record on the
    /// current map.
    async fn nb_records(&self) -> usize {
        self.lock().await.nb_records()
    }

    /// Returns the map rank of the specified player, which is the
    /// rank of that player's personal best in the ranking of all
    /// records on the current map. Returns `None` if that player
    /// has not set a record.
    async fn map_rank(&self, player_uid: i32) -> Option<usize> {
        self.lock()
            .await
            .pb(player_uid)
            .map(|rec| rec.map_rank as usize)
    }
}

pub struct RecordState {
    /// The number of players that have set a record on the current map.
    nb_records: usize,

    /// A detailed copy of the top 1 record, or `None` if no player
    /// has set a record on the current map.
    top_record: Option<RecordDetailed>,

    /// A limited, ranked list of the top records on the current map.
    top_records: Vec<Record>,

    /// Maps player UID to their personal best on the current map.
    pbs: HashMap<i32, RecordDetailed>,

    /// Maps player UID to the recorded sector data in their current run.
    run_sectors: HashMap<i32, Vec<RecordSector>>,
}

impl RecordState {
    fn init() -> Self {
        RecordState {
            nb_records: 0,
            top_record: None,
            top_records: vec![],
            pbs: HashMap::new(),
            run_sectors: HashMap::new(),
        }
    }

    /// The personal best of the specified player, on the current map, or
    /// `None` if they have not set a record yet.
    pub fn pb(&self, player_uid: i32) -> Option<&RecordDetailed> {
        self.pbs.get(&player_uid)
    }

    /// Iterate through the records of only connected players.
    pub fn playing_pbs(&self) -> impl Iterator<Item = &RecordDetailed> {
        self.pbs.values()
    }

    /// Returns a number of top records on the current map.
    /// The number is determined by how many records we want
    /// to display in-game. The vector is sorted from better
    /// to worse.
    pub fn top_records(&self) -> &Vec<Record> {
        &self.top_records
    }

    /// Returns the top record on the current map.
    pub fn top_record(&self) -> &Option<RecordDetailed> {
        &self.top_record
    }

    /// The number of players that have set a record on the
    /// current map.
    pub fn nb_records(&self) -> usize {
        self.nb_records
    }

    /// Without inserting the given record, return the map rank it would achieve,
    /// if it were inserted. Returns `None` if the record would not enter the
    /// top records that are cached in this state.
    pub fn pos_preview(&self, rec: &RecordEvidence) -> Option<usize> {
        if self.top_records.is_empty() {
            Some(1)
        } else {
            self.top_records
                .iter()
                .position(|other| rec.millis < other.millis)
                .map(|idx| idx + 1)
        }
    }

    fn upsert_record(&mut self, player_uid: i32, record: &RecordDetailed) {
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
            let ranking_rec = Record {
                player_login: record.player_login.clone(),
                player_nick_name: record.player_nick_name.clone(),
                millis: record.millis,
                timestamp: SystemTime::now(),
            };
            self.top_records.insert(idx, ranking_rec);
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
    state: Arc<RwLock<RecordState>>,
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
            state: Arc::new(RwLock::new(RecordState::init())),
        };

        if let Some(map) = live_playlist.current_map().await {
            controller.load_for_map(&map).await;
        }

        controller
    }

    /// Load a player's personal best when they join, or unload it when they leave.
    pub async fn load_for_player(&self, ev: &PlayerDiff) {
        use PlayerDiff::*;

        let map_uid = match self.live_playlist.current_map_uid().await {
            Some(uid) => uid,
            None => return,
        };

        match ev {
            AddPlayer(info) | AddSpectator(info) | AddPureSpectator(info) => {
                let pb = self
                    .db
                    .player_record(&map_uid, &info.login)
                    .await
                    .expect("failed to load player PB");
                if let Some(pb) = pb {
                    self.state.write().await.pbs.insert(info.uid, pb);
                }
            }
            RemovePlayer(info) | RemoveSpectator(info) | RemovePureSpectator(info) => {
                let mut state = self.state.write().await;
                state.pbs.remove(&info.uid);
                state.run_sectors.remove(&info.uid);
            }
            _ => {}
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

        let mut state = self.state.write().await;
        state.run_sectors.clear();
        state.top_record = top1;
        state.top_records = top_records;
        state.nb_records = nb_records;
        state.pbs.clear();

        let live_players = self.live_players.lock().await;
        for player_info in live_players.info_all() {
            let maybe_pb = self
                .db
                .player_record(&loaded_map.uid, &player_info.login)
                .await
                .expect("failed to load player PB");
            if let Some(pb) = maybe_pb {
                state.pbs.insert(player_info.uid, pb);
            }
        }
    }

    /// Add new sector data for a player's current run.
    pub async fn update_run(&self, ev: &CheckpointEvent) {
        if let Some(player_info) = self.live_players.info(&ev.player_login).await {
            let mut state = self.state.write().await;
            state
                .run_sectors
                .entry(player_info.uid)
                .or_insert_with(Vec::new)
                .push(RecordSector {
                    index: ev.cp_index,
                    cp_millis: ev.race_time_millis,
                    cp_speed: ev.speed,
                    cp_distance: ev.distance,
                });
        }
    }

    /// Produce a record from the collected sector data at the end of a run.
    /// If that run is the player's new personal best, update the map records.
    ///
    /// A new record will be stored in the database, including its
    /// validation replay. If it is the new top 1 record, its
    /// ghost replay will also be stored.
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

        let mut state = self.state.write().await;

        let prev_pb = state.pb(player.uid);
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

        // If we cannot get a validation replay, we cannot count the record.
        let validation = match self.server.validation_replay(&finish_ev.player_login).await {
            Ok(data) => data,
            Err(fault) => {
                log::error!("cannot get validation replay: {:?}", fault);
                return None;
            }
        };

        let sectors = state
            .run_sectors
            .remove(&player.uid)
            .expect("no sector data");
        let cp_millis = sectors.iter().map(|s| s.cp_millis).collect();

        let mut evidence = RecordEvidence {
            player_login: player.login.clone(),
            map_uid,
            millis: finish_ev.race_time_millis,
            validation,
            ghost: None,
            timestamp: SystemTime::now(),
            sectors,
        };

        // We already know the rank of the new record if it is better
        // than at least one of the cached records. Otherwise,
        // we have to look it up in the database.
        let new_pos: usize = match state.pos_preview(&evidence) {
            Some(pos) => pos,
            None => self
                .db
                .record_preview(&evidence)
                .await
                .expect("failed to check record rank") as usize,
        };

        // Store ghost replays for the very best records.
        if new_pos <= MAX_GHOST_REPLAY_RANK {
            match self.server.ghost_replay(&finish_ev.player_login).await {
                Ok(Ok(ghost)) => {
                    evidence.ghost = Some(ghost);
                }
                Ok(Err(io_err)) => log::error!("cannot get ghost replay: {:?}", io_err),
                Err(fault) => log::error!("cannot get ghost replay: {:?}", fault),
            }
        }

        // Remember record in the database.
        self.db
            .upsert_record(&evidence)
            .await
            .expect("failed to update player PB");

        let record = RecordDetailed {
            map_rank: new_pos as i64,
            player_login: player.login.clone(),
            player_nick_name: player.nick_name.clone(),
            timestamp: evidence.timestamp,
            millis: evidence.millis,
            cp_millis,
        };

        // Remember record in the cache.
        state.upsert_record(player.uid, &record);

        let pos_gained = match prev_pb_pos {
            Some(p) => p - new_pos,
            None if state.nb_records == 1 => 1,
            None => state.nb_records - new_pos,
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
            self.state.write().await.run_sectors.remove(&player_uid);
        }
    }
}

#[async_trait]
impl LiveRecords for RecordController {
    async fn lock(&self) -> RwLockReadGuard<'_, RecordState> {
        self.state.read().await
    }
}
