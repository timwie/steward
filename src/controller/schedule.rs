use std::cmp::{max, min};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use futures::future::join_all;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::controller::{LiveConfig, LivePlaylist, LiveQueue, LiveRecords, PublicConfig};
use crate::database::Database;
use crate::event::PlaylistDiff;
use crate::server::Server;

/// Use to lookup when a playlist map will be played.
#[async_trait]
pub trait LiveSchedule: Send + Sync {
    /// Returns the current *expected* duration until the specified map is
    /// played on the server, or `None` if that map is not in the playlist.
    /// The default duration (zero) signals that the map is currently being played.
    async fn time_until_played(&self, map_uid: &str) -> Option<Duration>;
}

struct ScheduleState {
    /// The moment the current map started.
    map_start_time: SystemTime,

    /// This list contains the reference times for each map in the playlist.
    /// These times should typically be the top record, or the author time,
    /// if no record was set yet.
    reference_millis: Vec<u64>,
}

#[derive(Clone)]
pub struct ScheduleController {
    state: Arc<RwLock<ScheduleState>>,
    server: Arc<dyn Server>,
    db: Arc<dyn Database>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_queue: Arc<dyn LiveQueue>,
    live_records: Arc<dyn LiveRecords>,
    live_config: Arc<dyn LiveConfig>,
}

impl ScheduleController {
    /// This will set the time limit for the current map.
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        server: &Arc<dyn Server>,
        db: &Arc<dyn Database>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_queue: &Arc<dyn LiveQueue>,
        live_records: &Arc<dyn LiveRecords>,
        live_config: &Arc<dyn LiveConfig>,
    ) -> Self {
        let playlist_state = live_playlist.lock().await;
        let reference_millis = join_all(playlist_state.maps.iter().map(|map| async move {
            let top1 = db
                .top_record(&map.uid)
                .await
                .expect("failed to load top record");
            top1.map(|rec| rec.millis).unwrap_or(map.author_millis) as u64
        }))
        .await;

        let state = ScheduleState {
            map_start_time: SystemTime::now(), // we don't know the actual time
            reference_millis,
        };

        let controller = ScheduleController {
            state: Arc::new(RwLock::new(state)),
            server: server.clone(),
            db: db.clone(),
            live_playlist: live_playlist.clone(),
            live_queue: live_queue.clone(),
            live_records: live_records.clone(),
            live_config: live_config.clone(),
        };

        controller.set_time_limit().await;

        controller
    }

    /// Sets the time limit for the current map.
    ///
    /// The time limit is based on either the author time, or the top record, if available.
    pub async fn set_time_limit(&self) {
        let idx = match self.live_playlist.current_index().await {
            Some(idx) => idx,
            None => return,
        };

        let mut schedule_state = self.state.write().await;
        let records_state = self.live_records.lock().await;

        let config = self.live_config.lock().await;
        let public_config = config.public();

        // Update in case a new record have been set the last time this map was played.
        if let Some(top_record) = &records_state.top_record {
            let top1_millis = top_record.millis as u64;
            std::mem::replace(&mut schedule_state.reference_millis[idx], top1_millis);
        }

        // Set the server's time limit
        let new_time_limit = self.to_limit(schedule_state.reference_millis[idx], &public_config);
        let mut mode_options = self.server.mode_options().await;
        mode_options.time_limit_secs = new_time_limit.as_secs() as i32;
        self.server.set_mode_options(&mode_options).await;
    }

    /// Update the cached reference times for playlist maps.
    pub async fn insert_or_remove(&self, diff: &PlaylistDiff) {
        let mut schedule_state = self.state.write().await;
        match diff {
            PlaylistDiff::AppendNew(map) => {
                schedule_state
                    .reference_millis
                    .push(map.author_millis as u64);
            }
            PlaylistDiff::Append(map) => {
                let top1 = self
                    .db
                    .top_record(&map.uid)
                    .await
                    .expect("failed to load top record");
                let millis = top1.map(|rec| rec.millis).unwrap_or(map.author_millis) as u64;
                schedule_state.reference_millis.push(millis);
            }
            PlaylistDiff::Remove { was_index, .. } => {
                schedule_state.reference_millis.remove(*was_index);
            }
        }
    }

    fn to_limit(&self, ref_millis: u64, public_config: &PublicConfig) -> Duration {
        const TIME_LIMIT_DIVIDER: u64 = 30 * 1000; // use steps of 30 seconds

        let n = TIME_LIMIT_DIVIDER;
        let i = ref_millis * public_config.time_limit_factor as u64;

        let rem = i % n;
        let limit = if rem > n / 2 {
            i + n - rem // round up
        } else {
            i - rem // round down
        };

        let limit = min(public_config.time_limit_max_secs as u64, limit);
        let limit = max(public_config.time_limit_min_secs as u64, limit);

        Duration::from_millis(limit)
    }
}

#[async_trait]
impl LiveSchedule for ScheduleController {
    async fn time_until_played(&self, map_uid: &str) -> Option<Duration> {
        let schedule_state = self.state.read().await;

        // The duration in between maps is the duration of the outro, plus
        // some time for map loading and intro. We'll choose five seconds for the latter.
        let duration_between_maps = self.live_config.outro_duration().await;
        let duration_between_maps = duration_between_maps + Duration::from_secs(5);

        let playlist_idx = match self.live_playlist.index_of(&map_uid).await {
            Some(idx) => idx,
            None => return None,
        };

        let curr_playlist_idx = self.live_playlist.current_index().await;
        if Some(playlist_idx) == curr_playlist_idx {
            return Some(Duration::default());
        }

        let config = self.live_config.lock().await;
        let public_config = config.public();

        let queue_state = self.live_queue.lock().await;
        let entries_ahead = queue_state
            .entries
            .iter()
            .take_while(|entry| entry.playlist_idx != playlist_idx);

        let mut result = Duration::default();

        // 1. add time until current map ends
        if let Some(idx) = curr_playlist_idx {
            let time_since_map_start = schedule_state.map_start_time.elapsed().unwrap_or_default();
            let ref_millis = schedule_state.reference_millis[idx];
            result += self.to_limit(ref_millis, &public_config);
            result -= time_since_map_start;
        }
        result += duration_between_maps;

        // 2. add time until the specified map starts
        for entry in entries_ahead {
            let ref_millis = schedule_state.reference_millis[entry.playlist_idx];
            result += self.to_limit(ref_millis, &public_config);
            result += duration_between_maps;
        }

        Some(result)
    }
}
