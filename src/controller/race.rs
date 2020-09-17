use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::controller::LivePlayers;
use crate::server::{CheckpointEvent, GameString, Scores, Server};

/// Use to lookup the ranking of the current race.
#[async_trait]
pub trait LiveRace: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, RaceState>;

    /// The ranking of the specified player in the current race,
    /// or `None` if they have not completed a run.
    async fn rank_of(&self, player_uid: i32) -> Option<usize>;
}

#[derive(Default)]
pub struct RaceState {
    pub ranking: Vec<RaceRank>,

    /// Lists UIDs of non-spectators that are still in the intro phase.
    /// The server does not wait for every player to start the race.
    pre_race: HashSet<i32>,

    pub warmup: bool,

    pub paused: bool,
}

#[derive(Clone)]
pub struct RaceController {
    state: Arc<RwLock<RaceState>>,
    live_players: Arc<dyn LivePlayers>,
}

#[derive(Clone)]
pub struct RaceRank {
    pub login: String,
    pub nick_name: GameString,
    pub millis: Option<usize>,
}

impl RaceController {
    pub async fn init(server: &Arc<dyn Server>, live_players: &Arc<dyn LivePlayers>) -> Self {
        let mut state: RaceState = Default::default();
        state.warmup = server.warmup_status().await.active;
        state.paused = server.pause_status().await.active;

        let controller = RaceController {
            state: Arc::new(RwLock::new(state)),
            live_players: live_players.clone(),
        };

        controller.reset().await;
        controller.set_scores(&server.scores().await).await;

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
        let mut race_state = self.state.write().await;
        race_state.pre_race.insert(uid)
    }

    /// Replace the entire ranking.
    pub async fn set_scores(&self, scores: &Scores) {
        let players_state = self.live_players.lock().await;
        let mut race_state = self.state.write().await;

        scores
            .players
            .iter()
            .filter_map(|game_score| {
                players_state.info(&game_score.login).map(|info| RaceRank {
                    login: info.login.clone(),
                    nick_name: game_score.nick_name.clone(),
                    millis: Some(game_score.best_time_millis as usize).filter(|millis| *millis > 0),
                })
            })
            .enumerate()
            .for_each(|(idx, score)| {
                race_state.ranking.retain(|s| s.login != score.login); // remove previous entry
                race_state.ranking.insert(idx, score);
            });
    }

    /// Set whether the current race is paused.
    pub async fn set_pause(&self, active: bool) -> bool {
        let mut race_state = self.state.write().await;
        let res = race_state.paused != active;
        race_state.paused = active;
        res
    }

    /// Set whether the current race is in warmup.
    pub async fn set_warmup(&self, active: bool) -> bool {
        let mut race_state = self.state.write().await;
        let res = race_state.warmup != active;
        race_state.warmup = active;
        res
    }

    /// Clear the ranking for a new race.
    pub async fn reset(&self) -> Vec<RaceRank> {
        let mut race_state = self.state.write().await;

        let res = race_state.ranking.drain(..).collect();
        race_state.pre_race.clear();

        self.live_players
            .info_all()
            .await
            .into_iter()
            .map(|info| RaceRank {
                login: info.login,
                nick_name: info.nick_name,
                millis: None,
            })
            .for_each(|score| race_state.ranking.push(score));

        res
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

        let mut race_state = self.state.write().await;
        if race_state.warmup || race_state.paused {
            return;
        }

        let prev_idx = race_state
            .ranking
            .iter()
            .position(|lr| lr.login == player_info.login);
        match prev_idx {
            Some(idx)
                if race_state.ranking[idx]
                    .millis
                    .map(|prev_millis| prev_millis < ev.race_time_millis as usize)
                    .unwrap_or(false) =>
            {
                return
            }
            Some(idx) => {
                race_state.ranking.remove(idx);
            }
            None => {}
        }

        let new_ranking = RaceRank {
            login: player_info.login,
            nick_name: player_info.nick_name,
            millis: Some(ev.race_time_millis as usize),
        };

        let new_idx = race_state
            .ranking
            .iter()
            .enumerate()
            .find(|(_, lr)| match lr.millis {
                None => true,
                Some(millis) => millis > ev.race_time_millis as usize,
            })
            .map(|(idx, _)| idx);

        match new_idx {
            Some(idx) => race_state.ranking.insert(idx, new_ranking),
            None => race_state.ranking.push(new_ranking),
        }
    }
}

#[async_trait]
impl LiveRace for RaceController {
    async fn lock(&self) -> RwLockReadGuard<'_, RaceState> {
        self.state.read().await
    }

    async fn rank_of(&self, player_uid: i32) -> Option<usize> {
        let player_login = match self.live_players.login(player_uid).await {
            Some(uid) => uid,
            None => return None,
        };
        self.lock()
            .await
            .ranking
            .iter()
            .enumerate()
            .find(|(_, lr)| lr.login == player_login)
            .map(|(idx, _)| idx + 1)
    }
}
