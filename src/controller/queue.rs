use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::{
    DEFAULT_MIN_RESTART_VOTE_RATIO, MAX_DISPLAYED_IN_QUEUE, MIN_RESTART_VOTE_RATIO_STEP,
};
use crate::controller::{ActivePreferenceValue, LivePlayers, LivePlaylist, LivePreferences};
use crate::event::{PlaylistDiff, QueueEntry, QueuePriority};
use crate::ingame::Server;

pub struct QueueState {
    /// Counts the number of times a map in the playlist was skipped.
    /// The number at the `n-th` index of this list is the skip count
    /// for the map at playlist index `n`. Whenever a map is played,
    /// its count is reset to zero, while the others are increased.
    times_skipped: Vec<usize>,

    /// A queue of playlist indices that point to maps that should
    /// be queued next, regardless of their priority. Only a map restart
    /// will take precedence.
    force_queue: VecDeque<usize>,

    /// The threshold percentage of players (not including spectators)
    /// that have to vote in favour of a restart to cause replaying a map.
    /// It is increased for each subsequent restart of the same map.
    min_restart_vote_ratio: f32,
}

impl QueueState {
    pub fn init(playlist_len: usize) -> Self {
        QueueState {
            times_skipped: vec![0; playlist_len],
            force_queue: VecDeque::new(),
            min_restart_vote_ratio: DEFAULT_MIN_RESTART_VOTE_RATIO,
        }
    }

    fn extend(&mut self) -> usize {
        let new_playlist_index = self.times_skipped.len();
        self.times_skipped.push(0);
        new_playlist_index
    }

    fn force_queue_front(&mut self, index: usize) {
        if self.force_queue.front() == Some(&index) {
            return;
        }
        self.force_queue.push_front(index);
    }

    fn force_queue_back(&mut self, index: usize) {
        if self.force_queue.back() == Some(&index) {
            return;
        }
        self.force_queue.push_back(index);
    }

    fn force_queue_pos(&self, index: usize) -> Option<usize> {
        self.force_queue.iter().position(|i| i == &index)
    }

    fn remove(&mut self, index: usize) {
        self.times_skipped.remove(index);
        self.force_queue.retain(|idx| *idx != index);
    }
}

#[derive(Clone)]
pub struct QueueController {
    state: Arc<RwLock<QueueState>>,
    server: Arc<dyn Server>,
    live_players: Arc<dyn LivePlayers>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_prefs: Arc<dyn LivePreferences>,
}

impl QueueController {
    pub async fn init(
        server: &Arc<dyn Server>,
        live_players: &Arc<dyn LivePlayers>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_prefs: &Arc<dyn LivePreferences>,
    ) -> Self {
        let state = QueueState::init(live_playlist.nb_maps().await);
        QueueController {
            state: Arc::new(RwLock::new(state)),
            server: server.clone(),
            live_players: live_players.clone(),
            live_playlist: live_playlist.clone(),
            live_prefs: live_prefs.clone(),
        }
    }

    /// The current minimum percentage of players with player slots that have
    /// to vote in favour of a restart. It is increased for each subsequent
    /// restart of the same map.
    pub async fn current_min_restart_vote_ratio(&self) -> f32 {
        self.state.read().await.min_restart_vote_ratio
    }

    /// Update the queue when maps are added or removed from the playlist.
    pub async fn insert_or_remove(&self, diff: &PlaylistDiff) {
        let mut state = self.state.write().await;
        match diff {
            PlaylistDiff::AppendNew(_) => {
                let new_idx = state.extend();
                state.force_queue_back(new_idx);
            }
            PlaylistDiff::Append(_) => {
                state.extend();
            }
            PlaylistDiff::Remove { was_index, .. } => {
                state.remove(*was_index);
            }
        }
    }

    /// Push the current map to the top of the queue.
    pub async fn force_restart(&self) -> bool {
        let curr_index = match self.live_playlist.current_index().await {
            Some(idx) => idx,
            None => return false,
        };

        let mut state = self.state.write().await;
        state.force_queue_front(curr_index);
        true
    }

    /// Queue the map at the specified playlist index ahead of other maps.
    ///
    /// This will put a map ahead of other maps, regardless of their priority.
    /// Other maps that are force-queued have a lower priority for each map
    /// that was force-queued before them.
    pub async fn force_queue(&self, playlist_index: usize) {
        let mut state = self.state.write().await;
        state.force_queue_back(playlist_index);
    }

    /// Returns a subset of the queue, ordered by priority.
    /// The first item in the list will be the next map.
    ///
    /// To calculate priorities, take into consideration:
    /// - all connected players' map preferences
    /// - for each map: the number of other maps that have been played
    ///   since last playing the former
    /// - the number of players voting for a restart
    pub async fn next_maps(&self) -> Vec<QueueEntry> {
        let mut state = self.state.write().await;
        let live_prefs = self.live_prefs.lock().await;
        let live_playlist = self.live_playlist.lock().await;

        let uid_playing = self.live_players.uid_playing().await;
        let active_votes = self.live_prefs.poll_restart().await;

        let maybe_curr_index = live_playlist.current_index();

        let voted_restart = {
            let nb_for_restart: Vec<&i32> = uid_playing.intersection(&active_votes).collect();
            let restart_vote_ratio = if nb_for_restart.is_empty() {
                0f32
            } else {
                uid_playing.len() as f32 / nb_for_restart.len() as f32
            };
            restart_vote_ratio >= state.min_restart_vote_ratio
        };

        // Every map was skipped once more, except for the current map, which
        // was skipped zero times.
        state.times_skipped.iter_mut().for_each(|n| *n += 1);
        if let Some(curr_index) = maybe_curr_index {
            if let Some(n) = state.times_skipped.get_mut(curr_index) {
                *n = 0;
            }
        }

        let pref_sum = |idx: usize| -> i32 {
            use ActivePreferenceValue::*;

            let map_uid = &live_playlist
                .at_index(idx)
                .expect("no map at this playlist index")
                .uid;
            let active_prefs = live_prefs.map_prefs(map_uid);
            active_prefs
                .iter()
                .map(|pv| match pv {
                    AutoPick => 1,
                    Pick => 1,
                    _ => -1,
                })
                .sum()
        };

        let mut priorities: Vec<(usize, QueuePriority)> = state
            .times_skipped
            .iter()
            .enumerate()
            .map(|(idx, skip_count)| {
                let prio = if voted_restart && Some(idx) == maybe_curr_index {
                    QueuePriority::VoteRestart
                } else if let Some(pos) = state.force_queue_pos(idx) {
                    QueuePriority::Force(pos)
                } else if Some(idx) == maybe_curr_index {
                    QueuePriority::NoRestart
                } else {
                    QueuePriority::Score(pref_sum(idx) + *skip_count as i32)
                };
                (idx, prio)
            })
            .collect();

        // If restart, increase the needed threshold to make another restart less
        // likely. Otherwise, reset it for the next map.
        if voted_restart {
            state.min_restart_vote_ratio += MIN_RESTART_VOTE_RATIO_STEP;
            if state.min_restart_vote_ratio > 1.0 {
                state.min_restart_vote_ratio = 1.0;
            }
        } else {
            state.min_restart_vote_ratio = DEFAULT_MIN_RESTART_VOTE_RATIO;
        }

        // If there is no restart, the first index in the force-queue,
        // if any, will be the index of the next map. Remove it, so that it is
        // not force-queued again.
        if !voted_restart {
            let _ = state.force_queue.pop_front();
        }

        // Sort by priority.
        priorities.sort_by(|(_, a), (_, b)| a.cmp(&b));
        priorities.truncate(MAX_DISPLAYED_IN_QUEUE);

        let (next_idx, _) = priorities.first().expect("playlist is empty");
        let is_restart = maybe_curr_index == Some(*next_idx);

        // Tell server the next map.
        if is_restart {
            self.server.restart_map().await;
        } else {
            self.server
                .playlist_change_next(*next_idx as i32)
                .await
                .expect("failed to set next playlist index");
        }

        priorities
            .into_iter()
            .map(|(idx, prio)| QueueEntry {
                map: live_playlist
                    .at_index(idx)
                    .expect("no map at this playlist index")
                    .clone(),
                priority: prio,
            })
            .collect()
    }
}
