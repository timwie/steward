use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::controller::LivePlayers;
use crate::ingame::{CheckpointEvent, GameString, Scores, Server};

/// Use to lookup the ranking of the current race.
#[async_trait]
pub trait LiveRace: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, RaceState>;

    /// The ranking of the specified player in the current race,
    /// or `None` if they have not completed a run.
    async fn rank_of(&self, player_uid: i32) -> Option<usize> {
        self.lock()
            .await
            .ranking
            .iter()
            .enumerate()
            .find(|(_, lr)| lr.uid == player_uid)
            .map(|(idx, _)| idx + 1)
    }
}

pub struct RaceState {
    ranking: Vec<RaceRank>,

    /// Lists UIDs of non-spectators that are still in the intro phase.
    /// The server does not wait for every player to start the race.
    pre_race: HashSet<i32>,
}

impl RaceState {
    fn init() -> Self {
        RaceState {
            ranking: vec![],
            pre_race: HashSet::new(),
        }
    }
}

#[derive(Clone)]
pub struct RaceController {
    state: Arc<RwLock<RaceState>>,
    live_players: Arc<dyn LivePlayers>,
}

#[derive(Clone)]
pub struct RaceRank {
    pub uid: i32,
    pub nick_name: GameString,
    pub millis: Option<usize>,
}

impl RaceController {
    pub async fn init(server: &Arc<dyn Server>, live_players: &Arc<dyn LivePlayers>) -> Self {
        let controller = RaceController {
            state: Arc::new(RwLock::new(RaceState::init())),
            live_players: live_players.clone(),
        };
        controller.reset().await;
        server.request_scores().await;
        controller
    }

    /// Signals that a player is past the intro stage,
    /// and is now part of the race. This function returns `false`
    /// if the specified player was already part of the race.
    pub async fn add_contestant(&self, login: &str) -> bool {
        let uid = match self.live_players.uid(login).await {
            Some(uid) => uid,
            None => return false,
        };
        self.state.write().await.pre_race.insert(uid)
    }

    /// Replace the entire ranking.
    pub async fn set(&self, scores: &Scores) {
        let live_players = self.live_players.lock().await;
        let mut state = self.state.write().await;

        scores
            .scores
            .iter()
            .filter_map(|game_score| {
                live_players.info(&game_score.login).map(|info| RaceRank {
                    uid: info.uid,
                    nick_name: game_score.nick_name.clone(),
                    millis: Some(game_score.best_time_millis as usize).filter(|millis| *millis > 0),
                })
            })
            .enumerate()
            .for_each(|(idx, score)| {
                state.ranking.retain(|s| s.uid != score.uid); // remove previous entry
                state.ranking.insert(idx, score);
            });
    }

    /// Clear the ranking for a new race.
    pub async fn reset(&self) {
        let mut state = self.state.write().await;

        state.ranking.clear();
        state.pre_race.clear();

        self.live_players
            .info_all()
            .await
            .into_iter()
            .map(|info| RaceRank {
                uid: info.uid,
                nick_name: info.nick_name,
                millis: None,
            })
            .for_each(|score| state.ranking.push(score));
    }

    /// Update the ranking if the finished line was crossed
    /// and the run improved a player's time.
    pub async fn update(&self, ev: &CheckpointEvent) {
        if !ev.is_finish {
            return;
        }
        let player_info = match self.live_players.info(&ev.player_login).await {
            Some(info) => info,
            None => return,
        };

        let mut state = self.state.write().await;

        let prev_idx = state
            .ranking
            .iter()
            .position(|lr| lr.uid == player_info.uid);
        match prev_idx {
            Some(idx)
                if state.ranking[idx]
                    .millis
                    .map(|prev_millis| prev_millis < ev.race_time_millis as usize)
                    .unwrap_or(false) =>
            {
                return
            }
            Some(idx) => {
                state.ranking.remove(idx);
            }
            None => {}
        }

        let new_ranking = RaceRank {
            uid: player_info.uid,
            nick_name: player_info.nick_name,
            millis: Some(ev.race_time_millis as usize),
        };

        let new_idx = state
            .ranking
            .iter()
            .enumerate()
            .find(|(_, lr)| {
                lr.millis.is_none() || lr.millis.unwrap() > ev.race_time_millis as usize
            })
            .map(|(idx, _)| idx);

        match new_idx {
            Some(idx) => state.ranking.insert(idx, new_ranking),
            None => state.ranking.push(new_ranking),
        }
    }
}

#[async_trait]
impl LiveRace for RaceController {
    async fn lock(&self) -> RwLockReadGuard<'_, RaceState> {
        self.state.read().await
    }
}
