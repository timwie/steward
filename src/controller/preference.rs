use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use futures::future::join_all;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::chat::PlayerMessage;
use crate::controller::{LiveChat, LivePlayers, LivePlaylist, PlayersState};
use crate::database::{DatabaseClient, History, Map, Preference, PreferenceValue};
use crate::event::{PlayerDiff, PlayerTransition, PlaylistDiff};
use crate::server::PlayerInfo;
use crate::widget::ActivePreferenceValue;

/// Use to lookup preferences of connected players.
#[async_trait]
pub trait LivePreferences: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, PreferencesState>;

    /// Map connected players' UID to their preferences for the current map.
    async fn current_map_prefs(&self) -> HashMap<i32, ActivePreferenceValue> {
        self.lock()
            .await
            .preferences
            .iter()
            .map(|(key, pref)| (key.player_uid, pref.value))
            .collect()
    }

    /// Collect preferences of connected players for the specified map.
    async fn map_prefs(&self, map_uid: &str) -> Vec<ActivePreferenceValue> {
        self.lock().await.map_prefs(map_uid)
    }

    /// Collect UIDs of non-spectating players that are in favor of
    /// restarting the current map.
    async fn poll_restart(&self) -> HashSet<i32>;
}

pub struct PreferencesState {
    /// The preferences of connected players (without spectators)
    /// for every map.
    preferences: HashMap<PlayerMapKey<'static>, ActivePreference>,

    /// The playing history of connected players for every map.
    history: HashMap<PlayerMapKey<'static>, History>,

    /// A set of UIDs of players (that have a player slot) that are
    /// in favor of a restart.
    restart_votes: HashSet<i32>,
}

/// Extends `Preference` by implicit preference values, like `AutoPick`.
#[derive(Clone)]
pub struct ActivePreference {
    pub player_uid: i32,
    pub map_uid: String,
    pub value: ActivePreferenceValue,
}

impl PreferencesState {
    pub fn init() -> Self {
        PreferencesState {
            restart_votes: HashSet::new(),
            history: HashMap::new(),
            preferences: HashMap::new(),
        }
    }

    /// Add or replace a player's map preference.
    pub fn add_pref(&mut self, pref: ActivePreference) {
        self.preferences.insert(
            PlayerMapKey::new(pref.player_uid, pref.map_uid.clone()),
            pref,
        );
    }

    pub fn add_history(&mut self, player_uid: i32, history: History) {
        self.history.insert(
            PlayerMapKey::new(player_uid, history.map_uid.clone()),
            history,
        );
    }

    /// Return the specified player's preference for the specified map.
    pub fn pref(&self, player_uid: i32, map_uid: &str) -> ActivePreferenceValue {
        let key = PlayerMapKey::new(player_uid, map_uid);
        self.preferences
            .get(&key)
            .map(|pref| pref.value)
            .unwrap_or(ActivePreferenceValue::None)
    }

    /// Return the specified player's history for the specified map.
    pub fn history<'a>(&'a self, player_uid: i32, map_uid: &'a str) -> Option<&'a History> {
        let key = PlayerMapKey::new(player_uid, map_uid);
        self.history.get(&key)
    }

    /// Count active map preferences of the specified, connected player.
    pub fn nb_player_prefs(&self, uid: i32) -> usize {
        self.preferences
            .iter()
            .filter(|(k, _)| k.player_uid == uid)
            .count()
    }

    /// Collect preferences of connected players for the specified map.
    pub fn map_prefs(&self, map_uid: &str) -> Vec<ActivePreferenceValue> {
        self.preferences
            .values()
            .filter_map(|pref| {
                if pref.map_uid == map_uid {
                    Some(pref.value)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlayerMapKey<'a> {
    pub player_uid: i32,
    pub map_uid: Cow<'a, str>, // can build with either &str or String
}

impl<'a> PlayerMapKey<'a> {
    fn new<S: Into<Cow<'a, str>>>(player_uid: i32, map_uid: S) -> Self {
        PlayerMapKey {
            player_uid,
            map_uid: map_uid.into(),
        }
    }
}

#[derive(Clone)]
pub struct PreferenceController {
    state: Arc<RwLock<PreferencesState>>,
    db: DatabaseClient,
    live_chat: Arc<dyn LiveChat>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_players: Arc<dyn LivePlayers>,
}

impl PreferenceController {
    pub async fn init(
        db: &DatabaseClient,
        live_chat: &Arc<dyn LiveChat>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_players: &Arc<dyn LivePlayers>,
    ) -> Self {
        let controller = PreferenceController {
            state: Arc::new(RwLock::new(PreferencesState::init())),
            db: db.clone(),
            live_chat: live_chat.clone(),
            live_playlist: live_playlist.clone(),
            live_players: live_players.clone(),
        };

        join_all(
            live_players
                .lock()
                .await
                .info_playing()
                .iter()
                .map(|info| controller.load_for_player(&info)),
        )
        .await;

        controller
    }

    async fn load_for_player(&self, player: &PlayerInfo) {
        let auto_picked_maps = self
            .db
            .maps_without_player_record(&player.login)
            .await
            .expect("failed to load player's maps without record");

        let prefs = self
            .db
            .player_preferences(&player.login)
            .await
            .expect("failed to load player preferences");

        let history = self
            .db
            .history(&player.login)
            .await
            .expect("failed to load player history");

        let mut preferences_state = self.state.write().await;
        let players_state = self.live_players.lock().await;

        for map_uid in auto_picked_maps {
            let pref = ActivePreference {
                player_uid: player.uid,
                map_uid,
                value: ActivePreferenceValue::AutoPick,
            };
            preferences_state.add_pref(pref);
        }
        for pref in prefs {
            if let Some(pref) = to_active_pref(pref, &players_state) {
                preferences_state.add_pref(pref);
            }
        }

        for entry in history {
            if let Some(player_uid) = players_state.uid(&entry.player_login) {
                preferences_state.add_history(*player_uid, entry);
            }
        }

        self.live_chat
            .tell(
                PlayerMessage::PreferenceReminder {
                    nb_active_preferences: preferences_state.nb_player_prefs(player.uid),
                },
                &player.login,
            )
            .await;
    }

    /// Load a player's data if they enter a player slot,
    /// and unload them when they leave their player slot.
    ///
    /// Any map a player has no record on will be given the `AutoPick`
    /// preference, unless another preference has been set.
    pub async fn update_for_player(&self, diff: &PlayerDiff) {
        use PlayerTransition::*;

        match diff.transition {
            AddPlayer | MoveToPlayer => {
                self.load_for_player(&diff.info).await;
            }
            RemovePlayer | MoveToSpectator | MoveToPureSpectator => {
                let mut preferences_state = self.state.write().await;
                preferences_state
                    .preferences
                    .retain(|k, _| k.player_uid != diff.info.uid);
                preferences_state
                    .history
                    .retain(|k, _| k.player_uid != diff.info.uid);
                preferences_state.restart_votes.remove(&diff.info.uid);
            }
            _ => {}
        }
    }

    /// Load player's data for that map if it is new or re-enabled,
    /// and unload them if it is removed.
    pub async fn update_for_map(&self, ev: &PlaylistDiff) {
        match ev {
            PlaylistDiff::Append(_) => {
                // we load data for every map anyway
            }
            PlaylistDiff::AppendNew(map) => {
                self.load_for_new_map(map).await;
            }
            PlaylistDiff::Remove { map, .. } => {
                let mut preferences_state = self.state.write().await;
                preferences_state
                    .preferences
                    .retain(|k, _| k.map_uid != map.uid);
                preferences_state
                    .history
                    .retain(|k, _| k.map_uid != map.uid);
            }
        }
    }

    /// Update a player's history, making the specified map their most recently
    /// played one.
    pub async fn update_history(&self, player_uid: i32, map_uid: &str) {
        let player_login = match self.live_players.login(player_uid).await {
            Some(login) => login,
            None => return,
        };

        // Update in database
        let now = Utc::now().naive_utc();
        self.db
            .add_history(&player_login, &map_uid, &now)
            .await
            .expect("failed to update player history");

        // Update in state
        let mut preferences_state = self.state.write().await;

        let key = PlayerMapKey::new(player_uid, map_uid.to_string());
        let map_last_played: Option<NaiveDateTime> = preferences_state
            .history
            .get(&key)
            .expect("failed to find map history")
            .last_played;

        if map_last_played.is_none() {
            // First time played: remove auto-pick
            if let Some(ActivePreference {
                value: ActivePreferenceValue::AutoPick,
                ..
            }) = preferences_state.preferences.get(&key)
            {
                preferences_state.preferences.remove(&key);
            }
        }

        for (k, v) in preferences_state.history.iter_mut() {
            // Only look at player's history on other maps
            if k.player_uid != player_uid || k.map_uid == map_uid {
                continue;
            }
            // If the specified map was played less recently than the map in 'v',
            // increase 'nb_maps_since'. (Some(_) > None)
            if map_last_played > v.last_played {
                v.nb_maps_since += 1;
            }
        }

        let map_history = preferences_state
            .history
            .get_mut(&key)
            .expect("failed to find map history");
        map_history.nb_maps_since = 0;
        map_history.last_played = Some(Utc::now().naive_utc());
    }

    async fn load_for_new_map(&self, map: &Map) {
        let mut preferences_state = self.state.write().await;
        let players_state = self.live_players.lock().await;
        let playlist_state = self.live_playlist.lock().await;

        players_state.info_playing().iter().for_each(|info| {
            let pref = ActivePreference {
                player_uid: info.uid,
                map_uid: map.uid.clone(),
                value: ActivePreferenceValue::AutoPick,
            };
            preferences_state.add_pref(pref)
        });

        players_state.info_playing().iter().for_each(|info| {
            let history = History {
                player_login: info.login.clone(),
                map_uid: map.uid.clone(),
                last_played: None,
                nb_maps_since: playlist_state.maps.len(),
            };
            preferences_state.add_history(info.uid, history);
        });
    }

    /// Update a player's map preference.
    pub async fn set_preference(&self, preference: ActivePreference) {
        let mut preferences_state = self.state.write().await;
        preferences_state.add_pref(preference.clone());

        let value = match preference.value {
            ActivePreferenceValue::Pick => PreferenceValue::Pick,
            ActivePreferenceValue::Veto => PreferenceValue::Veto,
            ActivePreferenceValue::Remove => PreferenceValue::Remove,
            _ => return,
        };

        let player_login = match self.live_players.login(preference.player_uid).await {
            Some(login) => login,
            None => return,
        };

        let db_pref = Preference {
            player_login,
            map_uid: preference.map_uid.clone(),
            value,
        };

        self.db
            .upsert_preference(&db_pref)
            .await
            .expect("failed to upsert preference of player");
    }

    /// Update a player's restart vote for the current map.
    pub async fn set_restart_vote(&self, player_uid: i32, vote: bool) {
        let mut preferences_state = self.state.write().await;
        if vote {
            preferences_state.restart_votes.insert(player_uid);
        } else {
            preferences_state.restart_votes.remove(&player_uid);
        }
    }

    /// Reset all restart votes when maps change.
    pub async fn reset_restart_votes(&self) {
        let mut preferences_state = self.state.write().await;
        preferences_state.restart_votes.clear()
    }
}

fn to_active_pref(pref: Preference, players: &PlayersState) -> Option<ActivePreference> {
    players.uid(&pref.player_login).map(|id| ActivePreference {
        player_uid: *id,
        map_uid: pref.map_uid,
        value: match pref.value {
            PreferenceValue::Pick => ActivePreferenceValue::Pick,
            PreferenceValue::Veto => ActivePreferenceValue::Veto,
            PreferenceValue::Remove => ActivePreferenceValue::Remove,
        },
    })
}

#[async_trait]
impl LivePreferences for PreferenceController {
    async fn lock(&self) -> RwLockReadGuard<'_, PreferencesState> {
        self.state.read().await
    }

    async fn poll_restart(&self) -> HashSet<i32> {
        let uid_playing = self.live_players.uid_playing().await;
        self.state
            .read()
            .await
            .restart_votes
            .iter()
            .copied()
            .filter(|uid| uid_playing.contains(&uid))
            .collect()
    }
}
