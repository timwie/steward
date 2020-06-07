use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::sync::RwLock;

use gbx::{Fault, PlayerInfo};

use crate::command::CommandOutput;
use crate::config::{
    MAX_DISPLAYED_MAP_RANKS, MAX_DISPLAYED_RACE_RANKS, MAX_DISPLAYED_SERVER_RANKS,
    START_HIDE_WIDGET_DELAY_MILLIS,
};
use crate::controller::*;
use crate::database::{Database, PreferenceValue, RecordDetailed};
use crate::event::*;
use crate::ingame::Server;
use crate::widget::*;

/// This controller collects cached & event data,
/// to build and send widgets to connected players.
#[derive(Clone)]
pub struct WidgetController {
    phase: Arc<RwLock<MapPhase>>,
    server: Arc<dyn Server>,
    db: Arc<dyn Database>,
    live_playlist: Arc<dyn LivePlaylist>,
    live_players: Arc<dyn LivePlayers>,
    live_race: Arc<dyn LiveRace>,
    live_records: Arc<dyn LiveRecords>,
    live_prefs: Arc<dyn LivePreferences>,
    live_server_ranking: Arc<dyn LiveServerRanking>,
}

/// May be used to select the widgets that will be sent to a player
/// when joining the server.
#[derive(PartialEq)]
enum MapPhase {
    Race,
    Outro { voting: bool, has_ranking: bool },
}

impl WidgetController {
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        server: &Arc<dyn Server>,
        db: &Arc<dyn Database>,
        live_playlist: &Arc<dyn LivePlaylist>,
        live_players: &Arc<dyn LivePlayers>,
        live_race: &Arc<dyn LiveRace>,
        live_records: &Arc<dyn LiveRecords>,
        live_server_ranking: &Arc<dyn LiveServerRanking>,
        live_prefs: &Arc<dyn LivePreferences>,
    ) -> Self {
        WidgetController {
            phase: Arc::new(RwLock::new(MapPhase::Race)),
            server: server.clone(),
            db: db.clone(),
            live_playlist: live_playlist.clone(),
            live_players: live_players.clone(),
            live_race: live_race.clone(),
            live_records: live_records.clone(),
            live_server_ranking: live_server_ranking.clone(),
            live_prefs: live_prefs.clone(),
        }
    }

    /// Add widgets that are displayed during the intro.
    pub async fn begin_intro(&self) {
        let mut phase = self.phase.write().await;
        *phase = MapPhase::Race;

        self.show_intro_widgets().await;
    }

    /// For the specified player, remove widgets that are displayed during the intro,
    /// and add widgets that are displayed during the race.
    pub async fn end_intro_for(&self, player_login: &str) {
        let live_players = self.live_players.lock().await;
        if let Some(info) = live_players.info(player_login) {
            self.hide_intro_widgets_for(info.uid).await;
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
    pub async fn begin_outro_and_vote(&self, ev: &VoteInfo) {
        let mut phase = self.phase.write().await;
        *phase = MapPhase::Outro {
            voting: true,
            has_ranking: false,
        };

        self.hide_race_widgets().await;
        self.show_outro_widgets(ev).await;
    }

    /// Remove widgets that are displayed during the vote,
    /// and add ones that display the vote's results.
    pub async fn end_vote(&self, queued_next: Vec<QueueEntry>) {
        let mut phase = self.phase.write().await;
        *phase = MapPhase::Outro {
            voting: false,
            has_ranking: match *phase {
                MapPhase::Outro { has_ranking, .. } => has_ranking,
                _ => false,
            },
        };

        self.hide_outro_poll().await;
        self.show_outro_poll_result(queued_next).await;
    }

    /// Hide all outro widgets ahead of loading the next map.
    pub async fn end_outro(&self) {
        self.hide_outro_widgets().await;
    }

    /// Display appropriate widgets for (new or transitioning) players
    /// and spectators.
    pub async fn refresh_for_player(&self, ev: &PlayerDiff) {
        use PlayerDiff::*;

        // Showing intro & outro widgets for joining players would require more
        // effort here: normally we collect the needed data, and display them
        // for all players at once. To keep things simple, we will not send them here -
        // it happens *at most* for a player while they are on the server,
        // and both intro & outro are short enough to justify this.

        if *self.phase.read().await != MapPhase::Race {
            return;
        }
        match ev {
            AddPlayer(info) | MoveToPlayer(info) => {
                self.show_race_widgets_for(info).await;
            }
            MoveToSpectator(info) | MoveToPureSpectator(info) => {
                self.hide_intro_widgets_for(info.uid).await; // in case they moved during the intro
                self.hide_race_widgets_for(info.uid).await; // in case they moved during the race
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

        if let PbDiff {
            prev_pos: None,
            new_record: Some(new_record),
            ..
        } = diff
        {
            // After players have set their first record on a map, send them
            // the sector diff widget.
            let live_players = self.live_players.lock().await;
            if let Some(info) = live_players.info(&new_record.player_login) {
                self.show_sector_diff_for(info).await;
            }
        }
    }

    /// Add or update widgets that should display server ranks.
    pub async fn refresh_server_ranking(&self, change: &ServerRankingDiff) {
        let mut phase = self.phase.write().await;

        if *phase == MapPhase::Race {
            let players = self.live_players.lock().await;
            for info in players.info_playing() {
                self.show_curr_rank_for(info).await;
            }
            return;
        }

        *phase = MapPhase::Outro {
            has_ranking: true,
            voting: match *phase {
                MapPhase::Outro { voting, .. } => voting,
                _ => false,
            },
        };

        self.show_outro_ranking(change).await;
    }

    /// Update any widget that displays the server's playlist.
    pub async fn refresh_playlist(&self) {
        let live_players = self.live_players.lock().await;
        for info in live_players.info_all() {
            self.show_toggle_menu_for(&info).await;
        }
    }

    /// Display a popup message to the specified player.
    pub async fn show_popup(&self, msg: CommandOutput<'_>, for_login: &str, mode: PopupMode) {
        if let Some(uid) = self.live_players.uid(for_login).await {
            let output = &msg.to_string();
            self.show_for(&PopupWidget { output, mode }, uid).await;
        }
    }

    async fn show_sector_diff_for(&self, player: &PlayerInfo) {
        let records = self.live_records.lock().await;

        let top_1: &RecordDetailed = match records.top_record() {
            Some(rec) => rec,
            None => return,
        };
        let pb: &RecordDetailed = match records.pb(player.uid) {
            Some(rec) => rec,
            None => return,
        };

        let widget = SectorDiffWidget {
            pb_millis: pb.millis as usize,
            pb_sector_millis: pb.sector_millis(),
            top1_millis: top_1.millis as usize,
            top1_sector_millis: top_1.sector_millis(),
        };

        self.show_for(&widget, player.uid).await;
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
            tokio::time::delay_for(delay).await;
            let res = server.send_manialink_to(&T::hidden(), for_uid).await;
            check_send_res(res);
        });
    }

    async fn show_race_widgets_for(&self, player: &PlayerInfo) {
        self.show_sector_diff_for(player).await;
        self.show_curr_rank_for(player).await;
        self.show_toggle_menu_for(player).await;
    }

    async fn hide_race_widgets_for(&self, for_uid: i32) {
        self.hide_for::<RunOutroWidget>(for_uid).await;
        self.hide_for::<SectorDiffWidget>(for_uid).await;
        self.hide_for::<LiveRanksWidget>(for_uid).await;
        self.hide_for::<ToggleMenuWidget>(for_uid).await;
    }

    async fn hide_race_widgets(&self) {
        self.hide::<RunOutroWidget>().await;
        self.hide::<SectorDiffWidget>().await;
        self.hide::<LiveRanksWidget>().await;
        self.hide::<ToggleMenuWidget>().await;
    }

    async fn show_outro_widgets(&self, ev: &VoteInfo) {
        self.show_outro_poll(ev).await;
        self.show_outro_scores().await;
        self.show(OutroServerRankingPlaceholder {}).await;
    }

    async fn hide_outro_widgets(&self) {
        self.hide::<OutroMapRankingsWidget>().await;
        self.hide::<OutroServerRankingWidget>().await;
        self.hide::<OutroQueueWidget>().await;
    }

    async fn hide_outro_poll(&self) {
        self.hide::<OutroQueueVoteWidget>().await;
    }

    async fn show_intro_widgets(&self) {
        let map = match self.live_playlist.current_map().await {
            Some(map) => map,
            None => return,
        };

        let author_nick_name = self
            .db
            .player(&map.author_login)
            .await
            .expect("failed to load player")
            .map(|p| p.nick_name);

        let nb_records = self.live_records.nb_records().await;
        let preference_counts: Vec<(PreferenceValue, usize)> = self
            .db
            .count_map_preferences(&map.uid)
            .await
            .expect("failed to count map preferences")
            .into_iter()
            .map(|(k, v)| (k, v as usize))
            .collect();

        let records = self.live_records.lock().await;
        let preferences = self.live_prefs.lock().await;

        // Do not show for pure spectators.
        let id_playing = self.live_players.uid_playing().await;

        for id in id_playing {
            let widget = IntroWidget {
                map_name: &map.name,
                map_author_login: &map.author_login,
                map_author_nick_name: author_nick_name.as_deref(),
                player_map_rank: records.pb(id).map(|rec| rec.map_rank as usize),
                max_map_rank: nb_records,
                player_preference: preferences.pref(id, &map.uid),
                preference_counts: preference_counts.clone(),
            };
            self.show_for(&widget, id).await;
        }
    }

    async fn hide_intro_widgets_for(&self, for_uid: i32) {
        self.hide_for_delayed::<IntroWidget>(
            for_uid,
            Duration::from_millis(START_HIDE_WIDGET_DELAY_MILLIS),
        )
        .await;
    }

    async fn show_toggle_menu_for(&self, player: &PlayerInfo) {
        let records = self.live_records.lock().await;
        let playlist = self.live_playlist.lock().await;
        let server_ranking = self.live_server_ranking.lock().await;

        let map_ranking = self.curr_map_ranking(&*records, &player).await;
        let server_ranking = self.curr_server_ranking(&*server_ranking, &player).await;
        let map_list = self.curr_map_list(&*playlist, &player).await;

        let menu = ToggleMenuWidget {
            map_ranking,
            server_ranking,
            map_list,
            max_displayed_race_ranks: MAX_DISPLAYED_RACE_RANKS,
        };
        self.show_for(&menu, player.uid).await;
    }

    async fn show_run_outro_for(&self, diff: &PbDiff) {
        let widget = RunOutroWidget {
            race_pos: self.live_race.rank_of(diff.player_uid).await.unwrap(),
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
                Duration::from_millis(START_HIDE_WIDGET_DELAY_MILLIS),
            )
            .await;
        }
    }

    async fn refresh_curr_rank(&self, diff: &PbDiff) {
        // update ranks for all players with records beneath this rank
        let max_pos_changed = match diff {
            PbDiff {
                new_record: Some(_),
                new_pos,
                ..
            } => new_pos,
            _ => return,
        };

        let players = self.live_players.lock().await;
        let records = self.live_records.lock().await;

        let need_update = records.playing_pbs().filter_map(|pb| {
            if pb.map_rank as usize >= *max_pos_changed {
                players.info(&pb.player_login)
            } else {
                None
            }
        });

        for info in need_update {
            self.show_curr_rank_for(info).await;
        }
    }

    async fn show_outro_poll(&self, ev: &VoteInfo) {
        let prefs = self.live_prefs.current_map_prefs().await;

        for id in self.live_players.uid_all().await {
            let widget = OutroQueueVoteWidget {
                min_restart_vote_ratio: ev.min_restart_vote_ratio,
                init_preference: prefs.get(&id).copied(),
            };
            self.show_for(&widget, id).await;
        }
    }

    async fn show_outro_poll_result(&self, queued_next: Vec<QueueEntry>) {
        let is_restart = match queued_next.first().map(|e| e.priority) {
            Some(QueuePriority::VoteRestart) => true,
            _ => false,
        };
        let next_maps = queued_next
            .iter()
            .map(|entry| OutroQueueEntry {
                map_name: &entry.map.name,
                priority: entry.priority,
            })
            .collect();
        let widget = OutroQueueWidget {
            is_restart,
            next_maps,
        };
        self.show(widget).await;
    }

    async fn show_outro_ranking(&self, change: &ServerRankingDiff) {
        let players = self.live_players.lock().await;
        let server_ranking = self.live_server_ranking.lock().await;

        for (id, diff) in change.diffs.iter() {
            let info = match players.uid_info(*id) {
                Some(info) => info,
                None => continue,
            };
            let widget = OutroServerRankingWidget {
                pos: diff.new_pos,
                max_pos: change.max_pos,
                wins_gained: diff.gained_wins,
                pos_gained: diff.gained_pos,
                server_ranking: self.curr_server_ranking(&*server_ranking, &info).await,
            };
            self.show_for(&widget, *id).await;
        }
    }

    async fn show_outro_scores(&self) {
        let players = self.live_players.lock().await;
        let records = self.live_records.lock().await;

        for info in players.info_all() {
            let widget = OutroMapRankingsWidget {
                map_ranking: self.curr_map_ranking(&*records, &info).await,
                max_displayed_race_ranks: MAX_DISPLAYED_RACE_RANKS,
            };
            self.show_for(&widget, info.uid).await;
        }
    }

    async fn show_curr_rank_for(&self, player: &PlayerInfo) {
        let records = self.live_records.lock().await;
        let server_ranking = self.live_server_ranking.lock().await;
        let players = self.live_players.lock().await;

        let maybe_pb = records.pb(player.uid);

        let server_rank = players
            .login(player.uid)
            .and_then(|login| server_ranking.rank_of(login))
            .map(|rank| rank.pos);

        let widget = LiveRanksWidget {
            pb_millis: maybe_pb.map(|rec| rec.millis as usize),
            map_rank: maybe_pb.map(|rec| rec.map_rank as usize),
            max_map_rank: records.nb_records(),
            server_rank,
            max_server_rank: server_ranking.max_pos(),
        };

        self.show_for(&widget, player.uid).await;
    }

    async fn curr_server_ranking<'a>(
        &self,
        server_ranking: &'a ServerRankingState,
        player: &'a PlayerInfo,
    ) -> ServerRanking<'a> {
        let to_entry = |r: &'a ServerRank| -> ServerRankingEntry {
            ServerRankingEntry {
                pos: r.pos,
                nick_name: &r.player_nick_name,
                nb_wins: r.nb_wins,
                nb_losses: r.nb_losses,
                is_own: r.player_login == player.login,
            }
        };

        let entries = server_ranking.top_ranks().map(to_entry).collect();

        let personal_entry = server_ranking.rank_of(&player.login).map(to_entry);

        ServerRanking {
            max_pos: server_ranking.max_pos(),
            entries,
            personal_entry,
            max_displayed_server_ranks: MAX_DISPLAYED_SERVER_RANKS,
        }
    }

    async fn curr_map_ranking<'a>(
        &self,
        records: &'a RecordState,
        player: &'a PlayerInfo,
    ) -> MapRanking<'a> {
        let map_ranks = records
            .top_records()
            .iter()
            .enumerate()
            .map(|(idx, rec)| MapRankingEntry {
                pos: idx + 1,
                nick_name: &rec.player_nick_name,
                millis: rec.millis as usize,
                timestamp: rec.timestamp,
                is_own: rec.player_login == player.login,
            })
            .collect();

        let personal_entry = records.pb(player.uid).map(|rec| MapRankingEntry {
            pos: rec.map_rank as usize,
            nick_name: &rec.player_nick_name,
            millis: rec.millis as usize,
            timestamp: rec.timestamp,
            is_own: rec.player_login == player.login,
        });

        MapRanking {
            max_displayed_map_ranks: MAX_DISPLAYED_MAP_RANKS,
            entries: map_ranks,
            personal_entry,
        }
    }

    async fn curr_map_list<'a>(
        &self,
        playlist: &'a PlaylistState,
        player: &'a PlayerInfo,
    ) -> MapList<'a> {
        let curr_map_uid = playlist.current_map().map(|m| &m.uid);
        let mut maps = join_all(playlist.maps().iter().map(|map| async move {
            let preference = self.live_prefs.lock().await.pref(player.uid, &map.uid);
            let nb_records = self
                .db
                .nb_records(&map.uid)
                .await
                .expect("failed to load number of records") as usize;
            let map_rank = self
                .db
                .player_record(&map.uid, &player.login)
                .await
                .expect("failed to load player PB")
                .map(|rec| rec.map_rank as usize);
            MapListEntry {
                map_uid: &map.uid,
                map_name: &map.name,
                map_author_login: &map.author_login,
                preference,
                nb_records,
                map_rank,
                added_since: map.added_since,
                is_current_map: Some(&map.uid) == curr_map_uid,
            }
        }))
        .await;
        maps.sort();
        MapList { maps }
    }
}

fn check_send_res(res: Result<(), Fault>) {
    match res {
        Ok(_) => {}
        Err(Fault { msg, .. }) if msg == "PlayerUId unknown." => {}
        _ => res.expect("failed to send widget"),
    }
}
