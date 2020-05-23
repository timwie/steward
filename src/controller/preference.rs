use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_repr::Serialize_repr;
use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::controller::{LiveChat, LivePlayers, LivePlaylist, PlayersState};
use crate::database::{Database, Map, Preference, PreferenceValue};
use crate::event::{PlayerDiff, PlaylistDiff};
use crate::ingame::PlayerInfo;
use crate::message::PlayerMessage;

/// Use to lookup preferences of connected players.
#[async_trait]
pub trait LivePreferences: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, PreferenceState>;

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

pub struct PreferenceState {
    /// The preferences of connected players (that have a player slot)
    /// that can influence the queue.
    preferences: HashMap<PreferenceKey<'static>, ActivePreference>,

    /// A set of UIDs of players (that have a player slot) that are
    /// in favor of a restart.
    restart_votes: HashSet<i32>,
}

/// Extends `Preference` by implicit preference values, like `AutoPick`.
pub struct ActivePreference {
    pub player_uid: i32,
    pub map_uid: String,
    pub value: ActivePreferenceValue,
}

#[derive(Serialize_repr, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum ActivePreferenceValue {
    // None = 0,
    AutoPick = 1,
    Pick = 2,
    Veto = 3,
    Remove = 4,
}

impl PreferenceState {
    pub fn init() -> Self {
        PreferenceState {
            restart_votes: HashSet::new(),
            preferences: HashMap::new(),
        }
    }

    /// Add or replace a player's map preference.
    pub fn add_pref(&mut self, pref: ActivePreference) {
        self.preferences.insert(
            PreferenceKey::new(pref.player_uid, pref.map_uid.clone()),
            pref,
        );
    }

    /// Return the preferences of the connected player with the given UID.
    pub fn pref(&self, player_uid: i32, map_uid: &str) -> Option<ActivePreferenceValue> {
        let key = PreferenceKey::new(player_uid, map_uid);
        self.preferences.get(&key).map(|pref| pref.value)
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
pub struct PreferenceKey<'a> {
    pub player_uid: i32,
    pub map_uid: Cow<'a, str>, // can build with either &str or String
}

impl<'a> PreferenceKey<'a> {
    fn new<S: Into<Cow<'a, str>>>(player_uid: i32, map_uid: S) -> Self {
        PreferenceKey {
            player_uid,
            map_uid: map_uid.into(),
        }
    }
}

#[derive(Clone)]
pub struct PreferenceController {
    state: Arc<RwLock<PreferenceState>>,
    db: Arc<dyn Database>,
    live_chat: Arc<dyn LiveChat>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_players: Arc<dyn LivePlayers>,
}

impl PreferenceController {
    pub async fn init(
        db: &Arc<dyn Database>,
        live_chat: &Arc<dyn LiveChat>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_players: &Arc<dyn LivePlayers>,
    ) -> Self {
        let controller = PreferenceController {
            state: Arc::new(RwLock::new(PreferenceState::init())),
            db: db.clone(),
            live_chat: live_chat.clone(),
            live_playlist: live_playlist.clone(),
            live_players: live_players.clone(),
        };
        for map in live_playlist.lock().await.maps() {
            controller.load_for_map(&map).await;
        }
        controller
    }

    async fn load_preferences(&self, player: &PlayerInfo) {
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

        let mut state = self.state.write().await;
        let players = self.live_players.lock().await;

        for map_uid in auto_picked_maps {
            let pref = ActivePreference {
                player_uid: player.uid,
                map_uid,
                value: ActivePreferenceValue::Pick,
            };
            state.add_pref(pref);
        }
        for pref in prefs {
            if let Some(pref) = to_active_pref(pref, &players) {
                state.add_pref(pref);
            }
        }

        self.live_chat
            .tell(
                PlayerMessage::PreferenceReminder {
                    nb_active_preferences: state.nb_player_prefs(player.uid),
                },
                &player.login,
            )
            .await;
    }

    /// Load a player's preferences if they enter a player slot,
    /// and unload them when they leave their player slot.
    ///
    /// Any map a player has no record on will be given the `AutoPick`
    /// preference, unless another preference has been set.
    pub async fn update_for_player(&self, ev: &PlayerDiff) {
        match ev {
            PlayerDiff::AddPlayer(info) | PlayerDiff::MoveToPlayer(info) => {
                self.load_preferences(info).await;
            }
            PlayerDiff::RemovePlayer(info) | PlayerDiff::MoveToSpectator(info) => {
                let mut state = self.state.write().await;
                state.preferences.retain(|k, _| k.player_uid != info.uid);
                state.restart_votes.remove(&info.uid);
            }
            _ => {}
        }
    }

    /// Load player's preferences for that map if it is new or re-enabled,
    /// and unload them if it is removed.
    pub async fn update_for_map(&self, ev: &PlaylistDiff) {
        match ev {
            PlaylistDiff::Remove { map, .. } => {
                let mut state = self.state.write().await;
                state.preferences.retain(|k, _| k.map_uid != map.uid);
            }
            PlaylistDiff::AppendNew(map) => {
                let mut state = self.state.write().await;
                let auto_picks = self
                    .live_players
                    .uid_all()
                    .await
                    .into_iter()
                    .map(|player_uid| ActivePreference {
                        player_uid,
                        map_uid: map.uid.clone(),
                        value: ActivePreferenceValue::AutoPick,
                    });
                for pref in auto_picks {
                    state.add_pref(pref);
                }
            }
            PlaylistDiff::Append(map) => {
                self.load_for_map(map).await;
            }
        }
    }

    async fn load_for_map(&self, map: &Map) {
        let mut state = self.state.write().await;
        let players = self.live_players.lock().await;

        let explicit = self
            .db
            .map_preferences(&map.uid)
            .await
            .expect("failed to load map preferences")
            .into_iter()
            .filter_map(|pref| to_active_pref(pref, &players));

        let auto_picks = self
            .db
            .players_without_map_record(&map.uid)
            .await
            .expect("failed to load players without map record")
            .into_iter()
            .filter_map(|player_login| players.uid(&player_login))
            .map(|player_uid| ActivePreference {
                player_uid: *player_uid,
                map_uid: map.uid.clone(),
                value: ActivePreferenceValue::AutoPick,
            });

        for pref in auto_picks.chain(explicit) {
            state.add_pref(pref);
        }
    }

    /// Update a player's map preference.
    pub async fn set_preference(&self, preference: Preference) {
        self.db
            .upsert_preference(&preference)
            .await
            .expect("failed to upsert preference of player");

        let mut state = self.state.write().await;
        let players = self.live_players.lock().await;

        if let Some(pref) = to_active_pref(preference, &players) {
            state.add_pref(pref);
        }
    }

    /// Clear a potential `AutoPick` preference when a player sets a record
    /// on the current map.
    pub async fn remove_auto_pick(&self, player_uid: i32) {
        let map_uid = match self.live_playlist.current_map_uid().await {
            Some(uid) => uid,
            None => return,
        };
        let key = PreferenceKey::new(player_uid, map_uid);

        let mut state = self.state.write().await;
        if let Some(ActivePreference {
            value: ActivePreferenceValue::AutoPick,
            ..
        }) = state.preferences.get(&key)
        {
            state.preferences.remove(&key);
        }
    }

    /// Update a player's restart vote for the current map.
    pub async fn set_restart_vote(&self, player_uid: i32, vote: bool) {
        let mut state = self.state.write().await;
        if vote {
            state.restart_votes.insert(player_uid);
        } else {
            state.restart_votes.remove(&player_uid);
        }
    }

    /// Reset all restart votes when maps change.
    pub async fn reset_restart_votes(&self) {
        self.state.write().await.restart_votes.clear()
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
    async fn lock(&self) -> RwLockReadGuard<'_, PreferenceState> {
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
