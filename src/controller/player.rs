use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::database::Database;
use crate::event::PlayerDiff;
use crate::server::{GameString, PlayerInfo, PlayerSlot};

/// Use to lookup information of connected players.
#[async_trait]
pub trait LivePlayers: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, PlayersState>;

    /// Return information about the player with the given login,
    /// or `None` if no such player is connected.
    async fn info(&self, login: &str) -> Option<PlayerInfo> {
        self.lock().await.info(&login).cloned()
    }

    /// Return information for all connected players and spectators.
    async fn info_all(&self) -> Vec<PlayerInfo> {
        self.lock().await.info_all().into_iter().cloned().collect()
    }

    /// Return the UID of the player with the given login,
    /// or `None` if no such player is connected.
    async fn uid(&self, login: &str) -> Option<i32> {
        self.lock().await.uid(&login).copied()
    }

    /// Return the UIDs for all connected players.
    async fn uid_all(&self) -> HashSet<i32> {
        self.lock().await.uid_all()
    }

    /// Return the UIDs for all connected players that are not spectating.
    async fn uid_playing(&self) -> HashSet<i32> {
        self.lock().await.playing.iter().copied().collect()
    }

    /// Return the login of the player with the specified UID, or `None` if no
    /// player with that UID is connected.
    async fn login(&self, player_uid: i32) -> Option<String> {
        self.lock()
            .await
            .login(player_uid)
            .map(|login| login.to_string())
    }

    /// Return the nick name of the player with the specified login, or `None` if no
    /// player with that login is connected.
    async fn nick_name(&self, login: &str) -> Option<GameString> {
        self.lock()
            .await
            .info(login)
            .map(|info| info.nick_name.clone())
    }
}

pub struct PlayersState {
    /// Cached player info for connected players.
    uid_to_info: HashMap<i32, PlayerInfo>,

    /// Maps player logins to their UIDs.
    login_to_uid: HashMap<String, i32>,

    /// Lists UIDs of players that are not spectating.
    playing: HashSet<i32>,

    /// Lists UIDs of players that are spectating, but have a player slot.
    spectating: HashSet<i32>,

    /// Lists UIDs of players that are spectating, and have *no* player slot.
    pure_spectating: HashSet<i32>,
}

impl PlayersState {
    fn init() -> Self {
        PlayersState {
            uid_to_info: HashMap::new(),
            login_to_uid: HashMap::new(),
            playing: HashSet::new(),
            spectating: HashSet::new(),
            pure_spectating: HashSet::new(),
        }
    }

    /// Return the UID of the player with the given login,
    /// or `None` if no such player is connected.
    pub fn uid(&self, login: &str) -> Option<&i32> {
        self.login_to_uid.get(login)
    }

    /// Return the UIDs for all connected players.
    pub fn uid_all(&self) -> HashSet<i32> {
        self.playing
            .union(&self.spectating)
            .copied()
            .collect::<HashSet<i32>>()
            .union(&self.pure_spectating)
            .copied()
            .collect()
    }

    /// Return detailed information for the given login.
    pub fn info(&self, login: &str) -> Option<&PlayerInfo> {
        self.login_to_uid
            .get(login)
            .and_then(|u| self.uid_to_info.get(u))
    }

    /// Return detailed information for the given login.
    pub fn uid_info(&self, player_uid: i32) -> Option<&PlayerInfo> {
        self.uid_to_info.get(&player_uid)
    }

    /// Return information for all connected players and spectators.
    pub fn info_all(&self) -> Vec<&PlayerInfo> {
        self.uid_to_info.values().collect()
    }

    /// Return information for all connected players that are not spectating.
    pub fn info_playing(&self) -> Vec<&PlayerInfo> {
        self.uid_to_info
            .values()
            .filter(|info| self.playing.contains(&info.uid))
            .collect()
    }

    pub fn login(&self, player_uid: i32) -> Option<&str> {
        self.uid_to_info
            .get(&player_uid)
            .map(|info| info.login.as_str())
    }
}

#[derive(Clone)]
pub struct PlayerController {
    state: Arc<RwLock<PlayersState>>,
    db: Arc<dyn Database>,
}

impl PlayerController {
    pub fn init(db: &Arc<dyn Database>) -> Self {
        PlayerController {
            state: Arc::new(RwLock::new(PlayersState::init())),
            db: db.clone(),
        }
    }

    /// Update a player's information.
    pub async fn update_player(&self, info: PlayerInfo) -> Option<PlayerDiff> {
        use PlayerDiff::*;

        if !info.is_player() {
            return None;
        }

        // If player disconnected
        if !info.has_joined() {
            return self.remove_player(&info.login).await;
        }

        let mut state = self.state.write().await;
        let uid = info.uid;

        // If player connected
        if !state.uid_to_info.contains_key(&uid) {
            self.db
                .upsert_player(&info)
                .await
                .expect("failed to upsert player data");

            let _ = state.login_to_uid.insert(info.login.to_string(), info.uid);
            let _ = state.uid_to_info.insert(info.uid, info.clone());
        }

        match info.slot() {
            PlayerSlot::None => None,
            PlayerSlot::Player => {
                if !state.playing.insert(uid) {
                    None
                } else if state.spectating.remove(&uid) || state.pure_spectating.remove(&uid) {
                    Some(MoveToPlayer(info))
                } else {
                    Some(AddPlayer(info))
                }
            }
            PlayerSlot::PlayerSpectator => {
                if !state.spectating.insert(uid) {
                    None
                } else if state.playing.remove(&uid) || state.pure_spectating.remove(&uid) {
                    Some(MoveToSpectator(info))
                } else {
                    Some(AddSpectator(info))
                }
            }
            PlayerSlot::PureSpectator => {
                if !state.pure_spectating.insert(uid) {
                    None
                } else if state.playing.remove(&uid) || state.spectating.remove(&uid) {
                    Some(MoveToPureSpectator(info))
                } else {
                    Some(AddPureSpectator(info))
                }
            }
        }
    }

    /// Remove a player's information.
    pub async fn remove_player(&self, login: &str) -> Option<PlayerDiff> {
        use PlayerDiff::*;

        let mut state = self.state.write().await;

        let uid = match state.login_to_uid.remove(&login.to_string()) {
            Some(uid) => uid,
            None => return None,
        };

        let info = match state.uid_to_info.remove(&uid) {
            Some(info) => info,
            None => return None,
        };

        if state.playing.remove(&uid) {
            return Some(RemovePlayer(info));
        }
        if state.spectating.remove(&uid) {
            return Some(RemoveSpectator(info));
        }
        if state.pure_spectating.remove(&uid) {
            return Some(RemovePureSpectator(info));
        }

        None
    }
}

#[async_trait]
impl LivePlayers for PlayerController {
    async fn lock(&self) -> RwLockReadGuard<'_, PlayersState> {
        self.state.read().await
    }
}
