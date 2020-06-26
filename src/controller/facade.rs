use std::sync::Arc;

use semver::Version;

use async_recursion::async_recursion;

use crate::chat::{
    AdminCommand, CommandConfirmResponse, CommandErrorResponse, CommandOutputResponse,
    CommandResponse, DangerousCommand, PlayerCommand, PlaylistCommandError, ServerMessage,
    SuperAdminCommand,
};
use crate::compat;
use crate::config::{Config, BLACKLIST_FILE, VERSION};
use crate::controller::*;
use crate::database::{Database, Preference};
use crate::event::{Command, ControllerEvent, PlaylistDiff, VoteInfo};
use crate::network::most_recent_controller_version;
use crate::server::{PlayerInfo, Server, ServerEvent};
use crate::widget::Action;

/// This facade hides all specific controllers behind one interface
/// that can react to server events.
#[derive(Clone)]
pub struct Controller {
    server: Arc<dyn Server>,
    db: Arc<dyn Database>,
    settings: SettingsController,
    chat: ChatController,
    playlist: PlaylistController,
    players: PlayerController,
    prefs: PreferenceController,
    queue: QueueController,
    schedule: ScheduleController,
    ranking: ServerRankController,
    records: RecordController,
    race: RaceController,
    widget: WidgetController,
}

impl Controller {
    pub async fn init(
        config: Config,
        server: Arc<dyn Server>,
        db: Arc<dyn Database>,
    ) -> Controller {
        // Lots and lots of dependency injection...

        // Controllers are up-casted to Live* traits, so that other controllers
        // can use cached data relevant for the current map/race/etc.
        // This facade will retain write access to update controller
        // states when receiving server events.

        // Using Arc<dyn T> everywhere to avoid lifetimes altogether.
        // We need 'static lifetimes, so that we can use controllers in Tokio tasks.
        // I *think* using something like Box<&'static dyn T> should be fine
        // as well, but I don't see any benefit.

        compat::prepare(&server, &db, &config).await;

        let settings = SettingsController::init(&server, config);
        let live_settings = Arc::new(settings.clone()) as Arc<dyn LiveSettings>;

        let chat = ChatController::init(&server, &live_settings);
        let msg_players = Arc::new(chat.clone()) as Arc<dyn LiveChat>;

        let playlist = PlaylistController::init(&server, &db, &live_settings).await;
        let live_playlist = Arc::new(playlist.clone()) as Arc<dyn LivePlaylist>;

        let players = PlayerController::init(&db);
        let live_players = Arc::new(players.clone()) as Arc<dyn LivePlayers>;

        let prefs =
            PreferenceController::init(&db, &msg_players, &live_playlist, &live_players).await;
        let live_prefs = Arc::new(prefs.clone()) as Arc<dyn LivePreferences>;

        let queue =
            QueueController::init(&server, &live_players, &live_playlist, &live_prefs).await;
        let live_queue = Arc::new(queue.clone()) as Arc<dyn LiveQueue>;

        let ranking = ServerRankController::init(&db, &live_players).await;
        let live_server_ranking = Arc::new(ranking.clone()) as Arc<dyn LiveServerRanking>;

        let records = RecordController::init(&server, &db, &live_playlist, &live_players).await;
        let live_records = Arc::new(records.clone()) as Arc<dyn LiveRecords>;

        let schedule = ScheduleController::init(
            &server,
            &db,
            &live_playlist,
            &live_queue,
            &live_records,
            &live_settings,
        )
        .await;
        let live_schedule = Arc::new(schedule.clone()) as Arc<dyn LiveSchedule>;

        let race = RaceController::init(&server, &live_players).await;
        let live_race = Arc::new(race.clone()) as Arc<dyn LiveRace>;

        let widget = WidgetController::init(
            &server,
            &db,
            &live_playlist,
            &live_players,
            &live_race,
            &live_records,
            &live_server_ranking,
            &live_prefs,
            &live_queue,
            &live_schedule,
        )
        .await;

        let init_players = server.players().await;

        let controller = Controller {
            server,
            db,
            settings,
            chat,
            playlist,
            players,
            prefs,
            queue,
            schedule,
            ranking,
            records,
            race,
            widget,
        };

        // It's easier to act as if players that were already connected just joined.
        for info in init_players {
            let ev = ServerEvent::PlayerInfoChanged { info };
            controller.on_server_event(ev).await;
        }

        controller
    }

    /// Server events are converted to controller events with the
    /// help of one or more controllers.
    pub async fn on_server_event(&self, event: ServerEvent) {
        log::debug!("{:#?}", &event);
        match event {
            ServerEvent::PlayerInfoChanged { info } => {
                if let Some(diff) = self.players.update_player(info).await {
                    let ev = ControllerEvent::NewPlayerList(diff);
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::PlayerDisconnect { login } => {
                if let Some(diff) = self.players.remove_player(&login).await {
                    let ev = ControllerEvent::NewPlayerList(diff);
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::MapLoad { is_restart } => {
                let loaded_map = self
                    .playlist
                    .current_map()
                    .await
                    .expect("server loaded map that was not in playlist");

                if is_restart {
                    let ev = ControllerEvent::EndOutro;
                    self.on_controller_event(ev).await;
                } else {
                    let ev = ControllerEvent::BeginMap { loaded_map };
                    self.on_controller_event(ev).await;
                }

                let ev = ControllerEvent::BeginIntro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::MapUnload => {
                let ev = ControllerEvent::EndOutro;
                self.on_controller_event(ev).await;

                let ev = ControllerEvent::EndMap;
                self.on_controller_event(ev).await;
            }

            ServerEvent::RaceEnd => {
                let vote_duration = self.settings.vote_duration().await;
                let min_restart_vote_ratio = self.queue.lock().await.min_restart_vote_ratio;
                let vote_info = VoteInfo {
                    duration: vote_duration,
                    min_restart_vote_ratio,
                };

                let outro_ev = ControllerEvent::BeginOutro { vote: vote_info };
                self.on_controller_event(outro_ev).await;

                // Delay for the duration of the vote.
                // Spawn a task to not block the callback loop.
                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    log::debug!("start vote");
                    tokio::time::delay_for(vote_duration).await;
                    log::debug!("end vote");

                    // Sort the queue, now that all restart votes have been cast.
                    // The next map is now at the top of the queue.
                    if let Some(diff) = controller.queue.sort_queue().await {
                        let ev = ControllerEvent::NewQueue(diff);
                        controller.on_controller_event(ev).await;
                    }

                    let end_vote_ev = ControllerEvent::EndVote;
                    controller.on_controller_event(end_vote_ev).await;
                });

                // Spawn a task to re-calculate the server ranking,
                // which could be expensive, depending on how we do it.
                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    let ranking_change = controller.ranking.update().await;
                    let new_ranking_ev = ControllerEvent::NewServerRanking(ranking_change);
                    controller.on_controller_event(new_ranking_ev).await;
                });
            }

            ServerEvent::RunStartline { player_login } => {
                // If this is the first time a player is at the start line,
                // their intro has just ended.
                let is_player_intro_end = self.race.add_contestant(&player_login).await;
                if is_player_intro_end {
                    let ev = ControllerEvent::EndIntro {
                        player_login: &player_login,
                    };
                    self.on_controller_event(ev).await;
                }

                let ev = ControllerEvent::BeginRun {
                    player_login: &player_login,
                };
                self.on_controller_event(ev).await;
            }

            ServerEvent::RunCheckpoint { event } if event.race_time_millis <= 0 => {
                // Invalid times (due to incoherence?) are apparently set to zero.
                // Ignore the run if it happens.
                self.records.reset_run(&event.player_login).await;
            }

            ServerEvent::RunCheckpoint { event } => {
                self.records.update_run(&event).await;

                if event.is_finish {
                    self.race.update(&event).await;

                    // Storing records involves file IO; run in separate task.
                    let controller = self.clone(); // 'self' with 'static lifetime
                    let _ = tokio::spawn(async move {
                        if let Some(pb_diff) = controller.records.end_run(&event).await {
                            let ev = ControllerEvent::EndRun { pb_diff };
                            controller.on_controller_event(ev).await;
                        }
                    });
                }
            }

            ServerEvent::PlayerAnswer {
                from_login, answer, ..
            } => {
                let action = Action::from_json(&answer);
                let ev = ControllerEvent::IssuedAction {
                    from_login: &from_login,
                    action,
                };
                self.on_controller_event(ev).await;
            }

            ServerEvent::PlayerChat {
                from_login,
                message,
                ..
            } => {
                if let Some(cmd) = self.chat.forward(&message, &from_login).await {
                    self.on_controller_event(ControllerEvent::IssuedCommand(cmd))
                        .await;
                }
            }

            ServerEvent::MapScores { scores } => {
                // This event is only useful when triggering it to get the score
                // at controller start. Otherwise, we can update it whenever
                // a player finishes a run.
                self.race.set(&scores).await;
            }

            ServerEvent::PlaylistChanged { curr_idx, .. } => {
                if let Some(curr_idx) = curr_idx {
                    self.playlist.set_index(curr_idx as usize).await;
                }
            }
        }
    }

    #[async_recursion]
    async fn on_controller_event(&self, event: ControllerEvent<'async_recursion>) {
        if let Some(server_msg) = ServerMessage::from_event(&event) {
            self.chat.announce(server_msg).await;
        }
        log::debug!("{:#?}", &event);
        match event {
            ControllerEvent::BeginRun { player_login } => {
                self.records.reset_run(&player_login).await;
                self.widget.end_run_outro_for(&player_login).await;
            }

            ControllerEvent::BeginMap { loaded_map } => {
                self.records.load_for_map(&loaded_map).await;
            }

            ControllerEvent::BeginIntro => {
                self.race.reset().await;

                self.schedule.set_time_limit().await;

                self.widget.begin_intro().await;
            }

            ControllerEvent::EndIntro { player_login } => {
                self.widget.end_intro_for(&player_login).await;
            }

            ControllerEvent::EndRun { pb_diff } => {
                self.widget.begin_run_outro_for(&pb_diff).await;
                self.widget.refresh_personal_best(&pb_diff).await;
                self.prefs.remove_auto_pick(pb_diff.player_uid).await;
            }

            ControllerEvent::BeginOutro { vote } => {
                self.widget.begin_outro_and_vote(&vote).await;
            }

            ControllerEvent::EndOutro => {
                self.widget.end_outro().await;
            }

            ControllerEvent::EndMap => {
                // Update the current map
                let next_index = self.server.playlist_next_index().await;
                self.playlist.set_index(next_index).await;

                // Re-sort the queue: the current map will move to the back.
                if let Some(diff) = self.queue.sort_queue().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;
                }
            }

            ControllerEvent::EndVote => {
                self.prefs.reset_restart_votes().await;

                let queue_preview = self.queue.peek().await;
                self.widget.end_vote(queue_preview).await;

                self.queue.pop_front().await;
            }

            ControllerEvent::NewQueue(diff) => {
                self.widget.refresh_queue_and_schedule(&diff).await;
            }

            ControllerEvent::NewPlayerList(diff) => {
                self.records.load_for_player(&diff).await;
                self.prefs.update_for_player(&diff).await;
                self.widget.refresh_for_player(&diff).await;
            }

            ControllerEvent::NewPlaylist(playlist_diff) => {
                // Update active preferences. This has to happen before re-sorting the queue.
                self.prefs.update_for_map(&playlist_diff).await;

                // Re-sort the map queue.
                let queue_diff = self.queue.insert_or_remove(&playlist_diff).await;
                let ev = ControllerEvent::NewQueue(queue_diff);
                self.on_controller_event(ev).await;

                // Add or remove the map from the schedule.
                self.schedule.insert_or_remove(&playlist_diff).await;

                // Update playlist UI.
                self.widget.refresh_playlist().await;

                // At this point, we could update the server ranking, since adding &
                // removing maps will affect it. But, doing so would give us weird
                // server ranking diffs during the outro. The diffs are only meaningful
                // if we calculate the ranking once per map.
            }

            ControllerEvent::NewServerRanking(change) => {
                self.widget.refresh_server_ranking(&change).await;
            }

            ControllerEvent::IssuedCommand(Command::Player { from, cmd }) => {
                self.on_cmd(&from, cmd).await
            }

            ControllerEvent::IssuedCommand(Command::Admin { from, cmd }) => {
                self.on_admin_cmd(&from, cmd).await
            }

            ControllerEvent::IssuedCommand(Command::SuperAdmin { from, cmd }) => {
                self.on_super_admin_cmd(&from, cmd).await
            }

            ControllerEvent::IssuedAction { from_login, action } => {
                if let Some(info) = self.players.info(&from_login).await {
                    self.on_action(&info, action).await;
                }
            }
        }
    }

    async fn on_action(&self, player: &PlayerInfo, action: Action<'_>) {
        use Action::*;

        match action {
            CommandConfirm => {
                if let Some(cmd) = self.chat.pop_unconfirmed_command(&player.login).await {
                    self.on_dangerous_cmd(&player.login, cmd).await;
                }
            }
            CommandCancel => {
                let _ = self.chat.pop_unconfirmed_command(&player.login).await;
            }
            SetPreference {
                map_uid,
                preference,
            } => {
                let pref = Preference {
                    map_uid: map_uid.to_string(),
                    player_login: player.login.clone(),
                    value: preference,
                };
                self.prefs.set_preference(pref).await;

                self.queue.sort_queue().await;
                if let Some(diff) = self.queue.sort_queue().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;
                }
            }
            VoteRestart { vote } => {
                self.prefs.set_restart_vote(player.uid, vote).await;
            }
        }
    }

    async fn on_cmd(&self, from_login: &str, cmd: PlayerCommand) {
        use PlayerCommand::*;

        match cmd {
            Help => {
                let msg = CommandResponse::Output(CommandOutputResponse::PlayerCommandReference);
                self.widget.show_popup(msg, from_login).await;
            }

            Info => {
                let controller = self.clone(); // 'self' with 'static lifetime
                let from_login = from_login.to_string(); // allow data to outlive the current scope
                let _ = tokio::spawn(async move {
                    let most_recent_controller_version = &most_recent_controller_version()
                        .await
                        .unwrap_or_else(|_| Version::new(0, 0, 0));
                    let config = &*controller.settings.lock_config().await;
                    let server_info = &controller.server.server_info().await;
                    let net_stats = &controller.server.net_stats().await;
                    let blacklist = &controller.server.blacklist().await;
                    let msg = CommandResponse::Output(CommandOutputResponse::Info {
                        controller_version: &VERSION,
                        most_recent_controller_version,
                        config,
                        server_info,
                        net_stats,
                        blacklist,
                    });
                    controller.widget.show_popup(msg, &from_login).await;
                });
            }
        }
    }

    async fn on_admin_cmd(&self, from_login: &str, cmd: AdminCommand<'_>) {
        use AdminCommand::*;

        let from_nick_name = match self.players.nick_name(from_login).await {
            Some(name) => name,
            None => return,
        };

        let or_nickname = |login: String| async move {
            self.db
                .player(&login)
                .await
                .expect("failed to load player")
                .map(|p| p.nick_name.formatted)
                .unwrap_or_else(|| login)
        };

        match cmd {
            Help => {
                let msg = CommandResponse::Output(CommandOutputResponse::AdminCommandReference);
                self.widget.show_popup(msg, from_login).await;
            }

            ListMaps => {
                let maps = self.db.maps().await.expect("failed to load maps");
                let msg =
                    CommandResponse::Output(CommandOutputResponse::MapList(maps.iter().collect()));
                self.widget.show_popup(msg, from_login).await;
            }

            ListPlayers => {
                let players = self.players.lock().await;
                let msg =
                    CommandResponse::Output(CommandOutputResponse::PlayerList(players.info_all()));
                self.widget.show_popup(msg, from_login).await;
            }

            PlaylistAdd { uid } => {
                self.on_playlist_cmd(from_login, self.playlist.add(&uid).await)
                    .await
            }

            PlaylistRemove { uid } => {
                self.on_playlist_cmd(from_login, self.playlist.remove(&uid).await)
                    .await
            }

            ImportMap { id } => {
                // Download maps in a separate task.
                let controller = self.clone(); // 'self' with 'static lifetime
                let id = id.to_string(); // allow data to outlive the current scope
                let from_login = from_login.to_string();
                let _ = tokio::spawn(async move {
                    controller
                        .on_playlist_cmd(&from_login, controller.playlist.import_map(&id).await)
                        .await;
                });
            }

            SkipCurrentMap => {
                self.server.end_map().await;
                self.chat
                    .announce(ServerMessage::CurrentMapSkipped {
                        admin_name: &from_nick_name.formatted,
                    })
                    .await;
            }

            RestartCurrentMap => {
                if let Some(diff) = self.queue.force_restart().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;

                    self.chat
                        .announce(ServerMessage::ForceRestart {
                            admin_name: &from_nick_name.formatted,
                        })
                        .await;
                }
            }

            ForceQueue { uid } => {
                let playlist = self.playlist.lock().await;
                let playlist_index = match playlist.index_of(uid) {
                    Some(idx) => idx,
                    None => {
                        let msg = CommandResponse::Error(CommandErrorResponse::UnknownMap);
                        self.widget.show_popup(msg, from_login).await;
                        return;
                    }
                };
                let map = playlist
                    .at_index(playlist_index)
                    .expect("no map at this playlist index");

                if let Some(diff) = self.queue.force_queue(playlist_index).await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;

                    self.chat
                        .announce(ServerMessage::ForceQueued {
                            admin_name: &from_nick_name.formatted,
                            map_name: &map.name.formatted,
                        })
                        .await;
                }
            }

            SetRaceDuration(secs) => {
                self.settings
                    .edit_config(|cfg: &mut Config| cfg.race_duration_secs = secs)
                    .await;
            }

            SetOutroDuration(secs) => {
                self.settings
                    .edit_config(|cfg: &mut Config| cfg.outro_duration_secs = secs)
                    .await;
            }

            BlacklistAdd { login } => {
                if login == from_login {
                    // Do not allow admins to blacklist themselves.
                    return;
                }

                let _ = self.players.remove_player(&login).await;
                let _ = self.server.kick_player(&login, Some("Blacklisted")).await;
                let _ = self.server.blacklist_add(&login).await;
                self.server
                    .save_blacklist(BLACKLIST_FILE)
                    .await
                    .expect("failed to save blacklist file");

                self.chat
                    .announce(ServerMessage::PlayerBlacklisted {
                        admin_name: &from_nick_name.formatted,
                        player_name: &or_nickname(login.to_string()).await,
                    })
                    .await;
            }

            BlacklistRemove { login } => {
                let blacklist = self.server.blacklist().await;
                if !blacklist.contains(&login.to_string()) {
                    let msg = CommandResponse::Error(CommandErrorResponse::UnknownBlacklistPlayer);
                    self.widget.show_popup(msg, from_login).await;
                    return;
                }

                let _ = self.server.blacklist_remove(&login).await;
                self.server
                    .save_blacklist(BLACKLIST_FILE)
                    .await
                    .expect("failed to save blacklist file");

                self.chat
                    .announce(ServerMessage::PlayerUnblacklisted {
                        admin_name: &from_nick_name.formatted,
                        player_name: &or_nickname(login.to_string()).await,
                    })
                    .await;
            }
        };
    }

    async fn on_super_admin_cmd(&self, from_login: &str, cmd: SuperAdminCommand) {
        use DangerousCommand::*;
        use SuperAdminCommand::*;

        match cmd {
            Help => {
                let msg =
                    CommandResponse::Output(CommandOutputResponse::SuperAdminCommandReference);
                self.widget.show_popup(msg, from_login).await;
            }

            Unconfirmed(DeleteMap { uid }) => {
                match self.db.map(&uid).await.expect("failed to load map") {
                    Some(map) if !map.in_playlist => {
                        let msg =
                            CommandResponse::Confirm(CommandConfirmResponse::ConfirmMapDeletion {
                                file_name: &map.file_name,
                            });
                        self.widget.show_popup(msg, from_login).await;
                    }
                    Some(_) => {
                        let msg =
                            CommandResponse::Error(CommandErrorResponse::CannotDeletePlaylistMap);
                        self.widget.show_popup(msg, from_login).await;
                    }
                    None => {
                        let msg = CommandResponse::Error(CommandErrorResponse::UnknownMap);
                        self.widget.show_popup(msg, from_login).await;
                    }
                }
            }

            Unconfirmed(DeletePlayer { login }) => {
                let blacklist = self.server.blacklist().await;
                if blacklist.contains(&login) {
                    let msg =
                        CommandResponse::Confirm(CommandConfirmResponse::ConfirmPlayerDeletion {
                            login: &login,
                        });
                    self.widget.show_popup(msg, from_login).await;
                } else {
                    let msg =
                        CommandResponse::Error(CommandErrorResponse::CannotDeleteWhitelistedPlayer);
                    self.widget.show_popup(msg, from_login).await;
                }
            }

            Unconfirmed(Shutdown) => {
                let msg = CommandResponse::Confirm(CommandConfirmResponse::ConfirmShutdown);
                self.widget.show_popup(msg, from_login).await;
            }
        }
    }

    async fn on_dangerous_cmd(&self, from_login: &str, cmd: DangerousCommand) {
        use DangerousCommand::*;

        log::warn!("{}> {:#?}", from_login, &cmd);

        let from_nick_name = match self.players.nick_name(from_login).await {
            Some(name) => name,
            None => return,
        };

        match cmd {
            DeleteMap { uid } => {
                let map = self
                    .db
                    .delete_map(&uid)
                    .await
                    .expect("failed to delete map")
                    .expect("map already deleted");

                // Delete file, otherwise the map will be scanned back into the
                // database at the next launch.
                let map_path = self.settings.maps_dir().await.join(map.file_name);
                if map_path.is_file() {
                    std::fs::remove_file(map_path).expect("failed to delete map file");
                }

                self.chat
                    .announce(ServerMessage::MapDeleted {
                        admin_name: &from_nick_name.formatted,
                        map_name: &map.name.formatted,
                    })
                    .await;
            }

            DeletePlayer { login } => {
                let maybe_player = self
                    .db
                    .delete_player(&login)
                    .await
                    .expect("failed to delete player");

                if maybe_player.is_none() {
                    let msg = CommandResponse::Error(CommandErrorResponse::UnknownPlayer);
                    self.widget.show_popup(msg, from_login).await;
                }
            }

            Shutdown => {
                self.server.stop_server().await;
            }
        }
    }

    async fn on_playlist_cmd(
        &self,
        from_login: &str,
        cmd_res: Result<PlaylistDiff, PlaylistCommandError>,
    ) {
        match cmd_res {
            Ok(change) => {
                let ev = ControllerEvent::NewPlaylist(change);
                self.on_controller_event(ev).await;
            }
            Err(err) => {
                let msg = CommandResponse::Error(CommandErrorResponse::InvalidPlaylistCommand(err));
                self.widget.show_popup(msg, from_login).await;
            }
        }
    }
}
