use std::cmp::Ordering;
use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::constants::{
    DEFAULT_MIN_RESTART_VOTE_RATIO, MAX_DISPLAYED_IN_QUEUE, MIN_RESTART_VOTE_RATIO_STEP,
};
use crate::controller::{LivePlayers, LivePlaylist, LivePreferences};
use crate::database::Map;
use crate::event::{PlaylistDiff, QueueDiff};
use crate::server::{Calls, ModeCalls, Server};
use crate::widget::ActivePreferenceValue;

/// Use to lookup the current queue, which is an ordering of the playlist.
#[async_trait]
pub trait LiveQueue: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, QueueState>;

    /// Returns the queue position of the specified map, or `None` if that map
    /// is not in the playlist.
    async fn pos(&self, map_uid: &str) -> Option<usize>;

    /// Returns a subset of the queue, ordered by priority.
    /// The first item in the list will be the next map.
    async fn peek(&self) -> Vec<QueueEntry>;
}

pub struct QueueState {
    /// An ordering of playlist indexes, sorted from highest to lowest
    /// priority.
    pub entries: Vec<QueueIndexEntry>,

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
    pub min_restart_vote_ratio: f32,
}

/// An entry in the map queue, which assigns a priority to the map in the playlist
/// at the given index.
#[derive(Debug)]
pub struct QueueIndexEntry {
    /// Position in the queue, starting at 0.
    /// The map at position 0 is the current map.
    /// The map at position 1 will be queued as the next map.
    pub pos: usize,

    /// The playlist index of the map represented by this entry.
    pub playlist_idx: usize,

    /// The priority of the map represented by this entry.
    /// The map with the highest priority will be queued as the next map.
    pub priority: QueuePriority,
}

/// An entry in the map queue, which assigns a priority to a map in the playlist.
#[derive(Debug)]
pub struct QueueEntry {
    /// A map in the queue.
    pub map: Map,

    /// Position in the queue, starting at 0.
    /// The map at position 0 is the current map.
    /// The map at position 1 will be queued as the next map.
    pub pos: usize,

    /// The queue priority of this map.
    /// The map with the highest priority will be queued as the next map.
    pub priority: QueuePriority,
}

/// When deciding the next map, each map is assigned a priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuePriority {
    /// Map was just played, and voted to restart.
    VoteRestart,

    /// Map was force-queued by an admin. The inner number is the
    /// amount of maps that were force-queued ahead of this map.
    /// This priority is used f.e. when importing new maps.
    Force(usize),

    /// Map has a calculated priority.
    Score(i32),

    /// Map was just played, and *not* voted to restart.
    /// Every other map has a higher priority.
    NoRestart,
}

impl Ord for QueuePriority {
    /// `VoteRestart < Force(x) < Force(x+1) < Score(y) < Score(y-1) < NoRestart`
    fn cmp(&self, other: &Self) -> Ordering {
        use QueuePriority::*;
        match (self, other) {
            (VoteRestart, VoteRestart) => Ordering::Equal,
            (NoRestart, NoRestart) => Ordering::Equal,
            (Score(a), Score(b)) => b.cmp(a), // higher score queued first
            (Force(a), Force(b)) => a.cmp(b), // lower pos queued first

            (VoteRestart, Score(_)) => Ordering::Less,
            (VoteRestart, NoRestart) => Ordering::Less,
            (Force(_), Score(_)) => Ordering::Less,
            (_, NoRestart) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for QueuePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl QueueState {
    pub fn init(playlist_len: usize) -> Self {
        QueueState {
            entries: vec![],
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

    fn force_queue_front(&mut self, index: usize) -> bool {
        if self.force_queue.front() == Some(&index) {
            return false;
        }
        self.force_queue.push_front(index);
        true
    }

    fn force_queue_back(&mut self, index: usize) -> bool {
        if self.force_queue.back() == Some(&index) {
            return false;
        }
        self.force_queue.push_back(index);
        true
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
    server: Server,
    live_players: Arc<dyn LivePlayers>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_prefs: Arc<dyn LivePreferences>,
}

impl QueueController {
    pub async fn init(
        server: &Server,
        live_players: &Arc<dyn LivePlayers>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_prefs: &Arc<dyn LivePreferences>,
    ) -> Self {
        let state = {
            let playlist_state = live_playlist.lock().await;
            QueueState::init(playlist_state.maps.len())
        };
        let controller = QueueController {
            state: Arc::new(RwLock::new(state)),
            server: server.clone(),
            live_players: live_players.clone(),
            live_playlist: live_playlist.clone(),
            live_prefs: live_prefs.clone(),
        };
        let _ = controller.sort_queue().await;
        controller
    }

    /// Update the queue when maps are added or removed from the playlist,
    /// and re-sort it.
    pub async fn insert_or_remove(&self, diff: &PlaylistDiff) -> QueueDiff {
        {
            let mut queue_state = self.state.write().await;
            match diff {
                PlaylistDiff::AppendNew(_) => {
                    let new_idx = queue_state.extend();
                    queue_state.force_queue_back(new_idx);
                }
                PlaylistDiff::Append(_) => {
                    queue_state.extend();
                }
                PlaylistDiff::Remove { was_index, .. } => {
                    queue_state.remove(*was_index);
                }
            }
        }
        self.sort_queue()
            .await
            .expect("playlist diff had no effect on queue")
    }

    /// Push the current map to the top of the queue, and re-sort it.
    /// Returns `false` if a restart was already forced.
    pub async fn force_restart(&self) -> Option<QueueDiff> {
        let curr_index = match self.live_playlist.current_index().await {
            Some(idx) => idx,
            None => return None,
        };
        let needs_sort = {
            let mut queue_state = self.state.write().await;
            queue_state.force_queue_front(curr_index)
        };
        if needs_sort {
            return self.sort_queue().await;
        }
        None
    }

    /// Put the map at the specified playlist index ahead of other maps,
    /// and re-sort the queue.
    /// Returns `false` if the specified map already was the last map to be force-queued.
    ///
    /// This will put a map ahead of other maps, regardless of their priority.
    /// Other maps that are force-queued have a lower priority for each map
    /// that was force-queued before them.
    pub async fn force_queue(&self, playlist_index: usize) -> Option<QueueDiff> {
        let needs_sort = {
            let mut queue_state = self.state.write().await;
            queue_state.force_queue_back(playlist_index)
        };
        if needs_sort {
            return self.sort_queue().await;
        }
        None
    }

    /// Re-order the queue, and take into consideration:
    /// - all connected players' map preferences
    /// - for each map: the number of other maps that have been played
    ///   since last playing the former
    /// - the number of players voting for a restart
    pub async fn sort_queue(&self) -> Option<QueueDiff> {
        let mut queue_state = self.state.write().await;
        let preferences_state = self.live_prefs.lock().await;
        let playlist_state = self.live_playlist.lock().await;

        let uid_playing = self.live_players.uid_playing().await;
        let active_votes = self.live_prefs.poll_restart().await;

        let maybe_curr_index = playlist_state.current_index;

        let voted_restart = {
            let nb_for_restart: Vec<&i32> = uid_playing.intersection(&active_votes).collect();
            let restart_vote_ratio = if nb_for_restart.is_empty() {
                0f32
            } else {
                uid_playing.len() as f32 / nb_for_restart.len() as f32
            };
            restart_vote_ratio >= queue_state.min_restart_vote_ratio
        };

        let pref_sum = |idx: usize| -> i32 {
            use ActivePreferenceValue::*;

            let map_uid = &playlist_state
                .at_index(idx)
                .expect("no map at this playlist index")
                .uid;
            let active_prefs = preferences_state.map_prefs(map_uid);
            active_prefs
                .iter()
                .map(|pv| match pv {
                    AutoPick => 1,
                    Pick => 1,
                    _ => -1,
                })
                .sum()
        };

        let mut priorities: Vec<(usize, QueuePriority)> = queue_state
            .times_skipped
            .iter()
            .enumerate()
            .map(|(idx, skip_count)| {
                let prio = if voted_restart && Some(idx) == maybe_curr_index {
                    QueuePriority::VoteRestart
                } else if let Some(pos) = queue_state.force_queue_pos(idx) {
                    QueuePriority::Force(pos)
                } else if Some(idx) == maybe_curr_index {
                    QueuePriority::NoRestart
                } else {
                    QueuePriority::Score(pref_sum(idx) + *skip_count as i32)
                };
                (idx, prio)
            })
            .collect();

        // Sort by priority.
        priorities.sort_by(|(_, a), (_, b)| a.cmp(&b));

        let new_entries: Vec<QueueIndexEntry> = priorities
            .into_iter()
            .enumerate()
            .map(|(idx, (playlist_idx, prio))| QueueIndexEntry {
                pos: idx,
                playlist_idx,
                priority: prio,
            })
            .collect();

        // Find the first index where a map has moved.
        let first_changed_idx = queue_state
            .entries
            .iter()
            .zip(new_entries.iter())
            .position(|(a, b)| a.playlist_idx != b.playlist_idx);

        // If there are new maps in the queue, the minimum changed index is equal
        // to the length of the old queue.
        let first_changed_idx =
            if first_changed_idx.is_none() && new_entries.len() > queue_state.entries.len() {
                Some(queue_state.entries.len())
            } else {
                first_changed_idx
            };

        queue_state.entries = new_entries;

        first_changed_idx.map(|idx| QueueDiff {
            first_changed_idx: idx,
        })
    }

    /// Tell the server to load the map at the top of the queue next,
    /// and returns it. This does *not* re-sort the queue.
    pub async fn pop_front(&self) -> Map {
        let mut queue_state = self.state.write().await;

        let maybe_curr_index = self.live_playlist.current_index().await;
        let head = queue_state.entries.first();
        let next_idx = head
            .map(|entry| entry.playlist_idx)
            .expect("queue is empty");
        let is_restart = Some(next_idx) == maybe_curr_index;

        // Every map was skipped once more, except for the current map, which
        // was skipped zero times.
        queue_state.times_skipped.iter_mut().for_each(|n| *n += 1);
        if let Some(curr_index) = maybe_curr_index {
            if let Some(n) = queue_state.times_skipped.get_mut(curr_index) {
                *n = 0;
            }
        }

        // If restart, increase the needed threshold to make another restart less
        // likely. Otherwise, reset it for the next map.
        if is_restart {
            queue_state.min_restart_vote_ratio += MIN_RESTART_VOTE_RATIO_STEP;
            if queue_state.min_restart_vote_ratio > 1.0 {
                queue_state.min_restart_vote_ratio = 1.0;
            }
        } else {
            queue_state.min_restart_vote_ratio = DEFAULT_MIN_RESTART_VOTE_RATIO;
        }

        // If the next map was force-queued, remove it from the force-queue.
        if Some(&next_idx) == queue_state.force_queue.front() {
            let _ = queue_state.force_queue.pop_front();
        }

        // Tell server the next map.
        if is_restart {
            self.server.restart_map().await;
        } else {
            self.server
                .playlist_change_next(next_idx as i32)
                .await
                .expect("failed to set next playlist index");
        }

        self.live_playlist
            .at_index(next_idx)
            .await
            .expect("queued bad playlist index")
    }
}

#[async_trait]
impl LiveQueue for QueueController {
    async fn lock(&self) -> RwLockReadGuard<'_, QueueState> {
        self.state.read().await
    }

    async fn pos(&self, map_uid: &str) -> Option<usize> {
        let queue_state = self.lock().await;
        self.live_playlist.index_of(map_uid).await.and_then(|idx| {
            queue_state.entries.iter().find_map(|entry| {
                if entry.playlist_idx == idx {
                    Some(entry.pos)
                } else {
                    None
                }
            })
        })
    }

    async fn peek(&self) -> Vec<QueueEntry> {
        let playlist_state = self.live_playlist.lock().await;
        self.lock()
            .await
            .entries
            .iter()
            .take(MAX_DISPLAYED_IN_QUEUE)
            .filter_map(|entry| {
                playlist_state
                    .at_index(entry.playlist_idx)
                    .cloned()
                    .map(|map| QueueEntry {
                        pos: entry.pos,
                        map,
                        priority: entry.priority,
                    })
            })
            .collect()
    }
}
