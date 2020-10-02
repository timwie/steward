use std::sync::Arc;

use chrono::Duration;
use futures::future::join_all;
use tokio::sync::RwLock;

use crate::chat::CommandResponse;
use crate::constants::{
    MAX_DISPLAYED_IN_QUEUE, MAX_DISPLAYED_RACE_RANKS, START_HIDE_WIDGET_DELAY_MILLIS,
};
use crate::controller::*;
use crate::database::{DatabaseClient, PreferenceValue};
use crate::event::*;
use crate::server::{Calls, Fault, PlayerInfo, Server};
use crate::widget::*;

/// This controller collects cached & event data,
/// to build and send widgets to connected players.
#[derive(Clone)]
pub struct WidgetController {
    state: Arc<RwLock<WidgetState>>,
    server: Server,
    db: DatabaseClient,
    live_config: Arc<dyn LiveConfig>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_players: Arc<dyn LivePlayers>,
    live_race: Arc<dyn LiveRace>,
    live_records: Arc<dyn LiveRecords>,
    live_prefs: Arc<dyn LivePreferences>,
    live_server_ranking: Arc<dyn LiveServerRanking>,
    live_queue: Arc<dyn LiveQueue>,
    live_schedule: Arc<dyn LiveSchedule>,
}

/// May be used to select the widgets that will be sent to a player
/// when joining the server.
#[derive(PartialEq)]
enum WidgetState {
    Race,
    Outro { voting: bool, has_ranking: bool },
}

impl WidgetController {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        server: &Server,
        db: &DatabaseClient,
        live_config: &Arc<dyn LiveConfig>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_players: &Arc<dyn LivePlayers>,
        live_race: &Arc<dyn LiveRace>,
        live_records: &Arc<dyn LiveRecords>,
        live_server_ranking: &Arc<dyn LiveServerRanking>,
        live_prefs: &Arc<dyn LivePreferences>,
        live_queue: &Arc<dyn LiveQueue>,
        live_schedule: &Arc<dyn LiveSchedule>,
    ) -> Self {
        let controller = WidgetController {
            state: Arc::new(RwLock::new(WidgetState::Race)),
            server: server.clone(),
            db: db.clone(),
            live_config: live_config.clone(),
            live_playlist: live_playlist.clone(),
            live_players: live_players.clone(),
            live_race: live_race.clone(),
            live_records: live_records.clone(),
            live_server_ranking: live_server_ranking.clone(),
            live_prefs: live_prefs.clone(),
            live_queue: live_queue.clone(),
            live_schedule: live_schedule.clone(),
        };

        for diff in live_players.lock().await.replay_diffs() {
            controller.refresh_for_player(&diff).await;
        }

        controller
    }

    /// Add widgets that are displayed during the intro.
    pub async fn begin_intro(&self) {
        // nothing to do here, we simply continue to display the outro widgets
    }

    /// For the specified player, remove widgets that are displayed during the intro,
    /// and add widgets that are displayed during the race.
    pub async fn end_intro_for(&self, player_login: &str) {
        let players_state = self.live_players.lock().await;
        if let Some(info) = players_state.info(player_login) {
            self.hide_outro_widgets_for(info.uid).await;
            self.show_race_widgets_for(info).await;
        }
    }

    /// Add widgets displayed in between player runs.
    pub async fn begin_run_outro_for(&self, diff: &PbDiff) {
        self.show_run_outro_for(diff).await;
    }

    /// Remove widgets displayed in between player runs.
    pub async fn end_run_outro_for(&self, player_login: &str) {
        self.hide_run_outro_for(player_login).await;
    }

    /// Remove race widgets, and add outro widgets, in particular
    /// those that are to be displayed during the vote.
    pub async fn begin_outro_and_vote(&self) {
        let mut widget_state = self.state.write().await;
        *widget_state = WidgetState::Outro {
            voting: true,
            has_ranking: false,
        };

        self.hide_race_widgets().await;
        self.show_outro_widgets().await;
    }

    /// Remove widgets displayed during the map's outro.
    pub async fn end_outro(&self) {
        let mut widget_state = self.state.write().await;
        *widget_state = WidgetState::Race;

        // nothing to do here, we simply continue to display the outro widgets during the intro
    }

    /// Remove widgets that are displayed during the vote,
    /// and add ones that display the vote's results.
    pub async fn end_vote(&self, queued_next: Vec<QueueEntry>) {
        let mut widget_state = self.state.write().await;
        *widget_state = WidgetState::Outro {
            voting: false,
            has_ranking: match *widget_state {
                WidgetState::Outro { has_ranking, .. } => has_ranking,
                _ => false,
            },
        };

        self.show_outro_queue(queued_next).await;
    }

    /// Display appropriate widgets for (new or transitioning) players
    /// and spectators.
    pub async fn refresh_for_player(&self, diff: &PlayerDiff) {
        use PlayerTransition::*;

        // Showing intro & outro widgets for joining players would require more
        // effort here: normally we collect the needed data, and display them
        // for all players at once. To keep things simple, we will not send them here -
        // it happens *at most* for a player while they are on the server,
        // and both intro & outro are short enough to justify this.

        if let AddPlayer | AddSpectator | AddPureSpectator = diff.transition {
            self.show_menu_for(&diff.info).await;
        }

        let is_race = *self.state.read().await == WidgetState::Race;
        if !is_race {
            return;
        }

        match diff.transition {
            AddPlayer | MoveToPlayer => {
                self.show_race_widgets_for(&diff.info).await;
            }
            MoveToSpectator | MoveToPureSpectator => {
                self.hide_race_widgets_for(diff.info.uid).await; // in case they moved during the race
            }
            _ => {}
        }
    }

    /// Update widgets after a player finishes a run:
    /// - If this is the first completed run by a player, update widgets
    ///   that show their personal best.
    /// - If this run improves their personal best, update widgets that
    ///   display their map rank. If this changes ranks of other connected
    ///   players, their widgets have to be updated as well.
    pub async fn refresh_personal_best(&self, diff: &PbDiff) {
        self.refresh_curr_rank(diff).await;
    }

    /// Add or update widgets that should display server ranks.
    pub async fn refresh_server_ranking(&self, change: &ServerRankingDiff) {
        let mut widget_state = self.state.write().await;

        if *widget_state == WidgetState::Race {
            let players_state = self.live_players.lock().await;
            for info in players_state.info_playing() {
                self.show_curr_rank_for(info).await;
            }
            return;
        }

        *widget_state = WidgetState::Outro {
            has_ranking: true,
            voting: match *widget_state {
                WidgetState::Outro { voting, .. } => voting,
                _ => false,
            },
        };

        self.show_outro_ranking(change).await;
    }

    /// Update any widget that displays the server's playlist.
    pub async fn refresh_playlist(&self) {
        let players_state = self.live_players.lock().await;
        for info in players_state.info_all() {
            self.show_playlist_for(&info).await;
        }
    }

    /// Update any widget that displays the server's map queue or schedule.
    pub async fn refresh_queue_and_schedule(&self, diff: &QueueDiff) {
        // TODO queue: update UI
        //  => if we use 'queue_pos' in the playlist, we could update it,
        //     but we shouldn't replace the entire widget for that,
        //     since it can happen frequently

        // Only refresh schedule if there are visible changes
        if diff.first_changed_idx < MAX_DISPLAYED_IN_QUEUE {
            self.refresh_schedule().await;
        }
    }

    /// Update any widget that displays the server's map schedule.
    pub async fn refresh_schedule(&self) {
        let players_state = self.live_players.lock().await;
        for info in players_state.info_all() {
            self.show_schedule_for(&info).await;
        }
    }

    /// Display a popup message to the specified player.
    pub async fn show_popup(&self, resp: CommandResponse<'_>, for_login: &str) {
        let mode = PopupMode::from(&resp);
        if let Some(uid) = self.live_players.uid(for_login).await {
            let output = &resp.to_string();
            self.show_for(&PopupWidget { output, mode }, uid).await;
        }
    }

    async fn show_for<T>(&self, widget: &T, for_uid: i32)
    where
        T: Widget,
    {
        let res = self
            .server
            .send_manialink_to(&widget.render(), for_uid)
            .await;
        check_send_res(res);
    }

    #[allow(dead_code)]
    async fn show<T>(&self, widget: T)
    where
        T: Widget,
    {
        self.server.send_manialink(&widget.render()).await;
    }

    async fn hide<T>(&self)
    where
        T: Widget,
    {
        self.server.send_manialink(&T::hidden()).await;
    }

    async fn hide_for<T>(&self, for_uid: i32)
    where
        T: Widget,
    {
        let res = self.server.send_manialink_to(&T::hidden(), for_uid).await;
        check_send_res(res);
    }

    async fn hide_for_delayed<T>(&self, for_uid: i32, delay: Duration)
    where
        T: Widget,
    {
        let server = self.server.clone();
        let _ = tokio::spawn(async move {
            tokio::time::delay_for(delay.to_std().expect("failed to hide widget with delay")).await;
            let res = server.send_manialink_to(&T::hidden(), for_uid).await;
            check_send_res(res);
        });
    }

    async fn show_race_widgets_for(&self, player: &PlayerInfo) {
        self.show_curr_rank_for(player).await;
    }

    async fn hide_race_widgets_for(&self, for_uid: i32) {
        self.hide_for::<RunOutroWidget>(for_uid).await;
        self.hide_for::<LiveRanksWidget>(for_uid).await;
    }

    async fn hide_race_widgets(&self) {
        self.hide::<RunOutroWidget>().await;
        self.hide::<LiveRanksWidget>().await;
    }

    async fn show_outro_widgets(&self) {
        let config = self.live_config.lock().await;
        let players_state = self.live_players.lock().await;
        let records_state = self.live_records.lock().await;

        let min_restart_vote_ratio = self.live_queue.lock().await.min_restart_vote_ratio;
        let prefs = self.live_prefs.current_map_prefs().await;

        for player in players_state.info_all() {
            let widget = OutroWidget {
                map_ranking: self.curr_map_ranking(&*records_state, &player).await,
                max_displayed_race_ranks: MAX_DISPLAYED_RACE_RANKS,
                min_restart_vote_ratio,
                init_preference: prefs.get(&player.uid).copied(),
                outro_duration_secs: config.timeattack.outro_duration_secs,
                vote_duration_secs: config.timeattack.vote_duration_secs(),
            };
            self.show_for(&widget, player.uid).await;
        }
    }

    async fn hide_outro_widgets_for(&self, for_uid: i32) {
        macro_rules! hide_after_delay {
            () => {};
            ($typ:tt, $($tail:tt)*) => {
                self.hide_for_delayed::<$typ>(
                    for_uid,
                    Duration::milliseconds(START_HIDE_WIDGET_DELAY_MILLIS),
                ).await;
                hide_after_delay!($($tail)*);
            }
        }

        hide_after_delay!(OutroWidget, OutroServerRankingWidget, OutroQueueWidget,);
    }

    async fn show_menu_for(&self, player: &PlayerInfo) {
        let server_ranking_state = self.live_server_ranking.lock().await;
        let records_state = self.live_records.lock().await;

        let server_ranking = self
            .curr_server_ranking(&*server_ranking_state, &player)
            .await;

        let map_ranking = self.curr_map_ranking(&*records_state, &player).await;

        let server_ranking_widget = ServerRankingWidget {
            ranking: server_ranking,
        };

        let map_ranking_widget = MapRankingWidget {
            ranking: map_ranking,
        };

        let menu = MenuWidget {};
        self.show_for(&menu, player.uid).await;
        self.show_for(&server_ranking_widget, player.uid).await;
        self.show_for(&map_ranking_widget, player.uid).await;

        self.show_playlist_for(&player).await;
        self.show_schedule_for(&player).await;
    }

    async fn show_playlist_for(&self, player: &PlayerInfo) {
        let playlist_state = self.live_playlist.lock().await;
        let preferences_state = self.live_prefs.lock().await;

        let playlist_widget = self
            .curr_map_list(&*playlist_state, &*preferences_state, &player)
            .await;

        self.show_for(&playlist_widget, player.uid).await;
    }

    async fn show_schedule_for(&self, player: &PlayerInfo) {
        let schedule_widget = ScheduleWidget {};
        self.show_for(&schedule_widget, player.uid).await;
    }

    async fn show_run_outro_for(&self, diff: &PbDiff) {
        let widget = RunOutroWidget {
            race_pos: self
                .live_race
                .rank_of(diff.player_uid)
                .await
                .expect("failed to get race rank of player"),
            pb_diff_millis: diff.millis_diff,
            record_pos: diff.new_pos,
            record_pos_gained: diff.pos_gained,
        };
        self.show_for(&widget, diff.player_uid).await;
    }

    async fn hide_run_outro_for(&self, player_login: &str) {
        if let Some(uid) = self.live_players.uid(player_login).await {
            self.hide_for_delayed::<RunOutroWidget>(
                uid,
                Duration::milliseconds(START_HIDE_WIDGET_DELAY_MILLIS),
            )
            .await;
        }
    }

    async fn refresh_curr_rank(&self, diff: &PbDiff) {
        // Nothing to do if PB not improved
        if diff.new_record.is_none() {
            return;
        }

        let players_state = self.live_players.lock().await;

        // Update for record setting player only, if they did not
        // improve their map rank.
        if diff.pos_gained == 0 {
            if let Some(info) = players_state.uid_info(diff.player_uid) {
                self.show_curr_rank_for(info).await;
            }
            return;
        }

        // Update ranks for all players with records beneath this rank.
        let max_pos_changed = diff.new_pos;

        let records_state = self.live_records.lock().await;

        let need_update = records_state.playing_pbs().filter_map(|pb| {
            if pb.map_rank as usize >= max_pos_changed {
                players_state.info(&pb.player_login)
            } else {
                None
            }
        });

        for info in need_update {
            self.show_curr_rank_for(info).await;
        }
    }

    async fn show_outro_queue(&self, queued_next: Vec<QueueEntry>) {
        let next_map = &queued_next.first().expect("empty queue").map;

        let next_map_author = self
            .db
            .player(&next_map.author_login)
            .await
            .expect("failed to load player")
            .map(|p| p.display_name)
            .unwrap_or_else(|| next_map.author_display_name.clone());

        let next_map_prefs: Vec<(PreferenceValue, usize)> = self
            .db
            .count_map_preferences(&next_map.uid)
            .await
            .expect("failed to count map preferences")
            .into_iter()
            .map(|(k, v)| (k, v as usize))
            .collect();

        let next_maps: Vec<OutroQueueEntry> = queued_next
            .iter()
            .map(|entry| OutroQueueEntry {
                map_name: &entry.map.name,
                annotation: match entry.priority {
                    QueuePriority::Score(_) => QueueEntryAnnotation::None,
                    QueuePriority::VoteRestart => QueueEntryAnnotation::Restart,
                    QueuePriority::Force(_) => QueueEntryAnnotation::Forced,
                    QueuePriority::NoRestart => QueueEntryAnnotation::PlayingNow,
                },
            })
            .collect();

        let is_restart = match queued_next.first().map(|e| e.priority) {
            Some(QueuePriority::VoteRestart) => true,
            _ => false,
        };

        let players_state = self.live_players.lock().await;
        let records_state = self.live_records.lock().await;
        let preferences_state = self.live_prefs.lock().await;

        for uid in players_state.uid_all() {
            let preview = MapPreview {
                map_name: &next_map.name,
                map_author_display_name: &next_map_author,
                player_map_rank: records_state.pb(uid).map(|rec| rec.map_rank as usize),
                max_map_rank: records_state.nb_records,
                player_preference: preferences_state.pref(uid, &next_map.uid),
                preference_counts: next_map_prefs.clone(),
                last_played: preferences_state
                    .history(uid, &next_map.uid)
                    .and_then(|h| h.last_played),
            };

            let widget = OutroQueueWidget {
                is_restart,
                next_maps: next_maps.to_vec(),
                next_map: preview,
            };
            self.show_for(&widget, uid).await;
        }
    }

    async fn show_outro_ranking(&self, change: &ServerRankingDiff) {
        let players_state = self.live_players.lock().await;
        let server_ranking_state = self.live_server_ranking.lock().await;

        for (id, diff) in change.diffs.iter() {
            let info = match players_state.uid_info(*id) {
                Some(info) => info,
                None => continue,
            };
            let widget = OutroServerRankingWidget {
                pos: diff.new_pos,
                max_pos: change.max_pos,
                wins_gained: diff.gained_wins,
                pos_gained: diff.gained_pos,
                server_ranking: self
                    .curr_server_ranking(&*server_ranking_state, &info)
                    .await,
            };
            self.show_for(&widget, *id).await;
        }
    }

    async fn show_curr_rank_for(&self, player: &PlayerInfo) {
        let players_state = self.live_players.lock().await;
        let records_state = self.live_records.lock().await;
        let server_ranking_state = self.live_server_ranking.lock().await;

        let maybe_pb = records_state.pb(player.uid);

        let server_rank = players_state
            .login(player.uid)
            .and_then(|login| server_ranking_state.rank_of(login))
            .map(|rank| rank.pos);

        let widget = LiveRanksWidget {
            pb_millis: maybe_pb.map(|rec| rec.millis as usize),
            top1_millis: records_state
                .top_record
                .as_ref()
                .map(|rec| rec.millis as usize),
            map_rank: maybe_pb.map(|rec| rec.map_rank as usize),
            max_map_rank: records_state.nb_records,
            server_rank,
            max_server_rank: server_ranking_state.max_pos(),
        };

        self.show_for(&widget, player.uid).await;
    }

    async fn curr_server_ranking<'a>(
        &self,
        server_ranking: &'a ServerRankingState,
        for_player: &'a PlayerInfo,
    ) -> ServerRanking<'a> {
        let to_entry = |r: &'a ServerRank| -> ServerRankingEntry {
            ServerRankingEntry {
                pos: r.pos,
                display_name: &r.player_display_name,
                nb_wins: r.nb_wins,
                nb_losses: r.nb_losses,
                is_own: r.player_login == for_player.login,
            }
        };

        let entries = server_ranking.top_ranks().map(to_entry).collect();

        let personal_entry = server_ranking.rank_of(&for_player.login).map(to_entry);

        ServerRanking {
            max_pos: server_ranking.max_pos(),
            entries,
            personal_entry,
        }
    }

    async fn curr_map_ranking<'a>(
        &self,
        records_state: &'a RecordsState,
        for_player: &'a PlayerInfo,
    ) -> MapRanking<'a> {
        let map_ranks = records_state
            .top_records
            .iter()
            .enumerate()
            .map(|(idx, rec)| MapRankingEntry {
                pos: idx + 1,
                display_name: &rec.player_display_name,
                millis: rec.millis as usize,
                timestamp: rec.timestamp,
                is_own: rec.player_login == for_player.login,
            })
            .collect();

        let personal_entry = records_state.pb(for_player.uid).map(|rec| MapRankingEntry {
            pos: rec.map_rank as usize,
            display_name: &rec.player_display_name,
            millis: rec.millis as usize,
            timestamp: rec.timestamp,
            is_own: rec.player_login == for_player.login,
        });

        MapRanking {
            entries: map_ranks,
            personal_entry,
            max_pos: records_state.nb_records,
        }
    }

    async fn curr_map_list<'a>(
        &self,
        playlist_state: &'a PlaylistState,
        prefs: &'a PreferencesState,
        for_player: &'a PlayerInfo,
    ) -> PlaylistWidget<'a> {
        let curr_map_uid = playlist_state.current_map().map(|m| &m.uid);
        let mut maps = join_all(playlist_state.maps.iter().map(|map| async move {
            let preference = prefs.pref(for_player.uid, &map.uid);
            let history = prefs.history(for_player.uid, &map.uid);
            let nb_records = self
                .db
                .nb_records(&map.uid, 0)
                .await
                .expect("failed to load number of records") as usize;
            let map_rank = self
                .db
                .player_record(&map.uid, &for_player.login, 0)
                .await
                .expect("failed to load player PB")
                .map(|rec| rec.map_rank as usize);
            PlaylistWidgetEntry {
                map_uid: &map.uid,
                map_name: &map.name,
                map_author_display_name: &map.author_display_name,
                preference,
                nb_records,
                map_rank,
                added_since: map.added_since,
                is_current_map: Some(&map.uid) == curr_map_uid,
                last_played: history.and_then(|h| h.last_played),
                queue_pos: self
                    .live_queue
                    .pos(&map.uid)
                    .await
                    .expect("failed to get queue priority"),
            }
        }))
        .await;
        maps.sort();
        PlaylistWidget { entries: maps }
    }
}

fn check_send_res(res: Result<(), Fault>) {
    match res {
        Ok(_) => {}
        Err(Fault { msg, .. }) if msg == "PlayerUId unknown." => {}
        _ => res.expect("failed to send widget"),
    }
}
