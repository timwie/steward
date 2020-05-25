use std::sync::Arc;

use async_recursion::async_recursion;

use crate::action::Action;
use crate::command::{AdminCommand, CommandOutput, PlayerCommand, PlaylistCommandError};
use crate::config::Config;
use crate::controller::*;
use crate::database::{Database, Preference};
use crate::event::{Command, ControllerEvent, PlaylistDiff, VoteInfo};
use crate::ingame::{Server, ServerEvent};
use crate::message::ServerMessage;
use gbx::PlayerInfo;

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

        let ranking = ServerRankController::init(&db, &live_players).await;
        let live_server_ranking = Arc::new(ranking.clone()) as Arc<dyn LiveServerRanking>;

        let records = RecordController::init(&server, &db, &live_playlist, &live_players).await;
        let live_records = Arc::new(records.clone()) as Arc<dyn LiveRecords>;

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
        log::debug!("{:?}", &event);
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

            ServerEvent::MapBegin { map: game_map } => {
                let loaded_map = self.playlist.set_current_index(&game_map).await;
                let ev = ControllerEvent::BeginIntro { loaded_map };
                self.on_controller_event(ev).await;
            }

            ServerEvent::MapEnd { .. } => {
                let ev = ControllerEvent::EndOutro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::RaceEnd => {
                let vote_duration = self.settings.vote_duration();
                let min_restart_vote_ratio = self.queue.current_min_restart_vote_ratio().await;
                let vote_info = VoteInfo {
                    duration: vote_duration,
                    min_restart_vote_ratio,
                };

                let outro_ev = ControllerEvent::BeginOutro { vote: vote_info };
                self.on_controller_event(outro_ev).await;
                {
                    // Delay for the duration of the vote.
                    // Spawn a task to not block the callback loop.
                    let controller = self.clone(); // 'self' with 'static lifetime
                    let _ = tokio::spawn(async move {
                        log::debug!("start vote");
                        tokio::time::delay_for(vote_duration).await;
                        log::debug!("end vote");
                        let queue_preview = controller.queue.next_maps().await;

                        let end_vote_ev = ControllerEvent::EndVote { queue_preview };
                        controller.on_controller_event(end_vote_ev).await;
                    });
                };
                {
                    // Spawn a task to re-calculate the server ranking,
                    // which could be expensive, depending on how we do it.
                    let controller = self.clone(); // 'self' with 'static lifetime
                    let _ = tokio::spawn(async move {
                        let ranking_change = controller.ranking.update().await;
                        let new_ranking_ev = ControllerEvent::NewServerRanking(ranking_change);
                        controller.on_controller_event(new_ranking_ev).await;
                    });
                }
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
        }
    }

    #[async_recursion]
    async fn on_controller_event(&self, event: ControllerEvent<'async_recursion>) {
        if let Some(server_msg) = ServerMessage::from_event(&event) {
            self.chat.announce(server_msg).await;
        }
        log::debug!("{:?}", &event);
        match event {
            ControllerEvent::BeginRun { player_login } => {
                self.records.reset_run(&player_login).await;
                self.widget.end_run_outro_for(&player_login).await;
            }

            ControllerEvent::BeginIntro { loaded_map } => {
                self.race.reset().await;
                self.records.load_for_map(&loaded_map).await;
                self.prefs.reset_restart_votes().await;
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

            ControllerEvent::EndVote { queue_preview } => {
                self.widget.end_vote(queue_preview).await;
            }

            ControllerEvent::NewPlayerList(diff) => {
                self.records.load_for_player(&diff).await;
                self.prefs.update_for_player(&diff).await;
                self.widget.refresh_for_player(&diff).await;
            }

            ControllerEvent::NewPlaylist(diff) => {
                self.queue.insert_or_remove(&diff).await;
                self.prefs.update_for_map(&diff).await;
                self.widget.refresh_playlist().await;
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

            ControllerEvent::IssuedAction { from_login, action } => {
                if let Some(info) = self.players.info(&from_login).await {
                    self.on_action(&info, action).await;
                }
            }
        }
    }

    async fn on_action(&self, player: &PlayerInfo, action: Action<'_>) {
        match action {
            Action::SetPreference {
                map_uid,
                preference,
            } => {
                let pref = Preference {
                    map_uid: map_uid.to_string(),
                    player_login: player.login.clone(),
                    value: preference,
                };
                self.prefs.set_preference(pref).await;
            }
            Action::VoteRestart { vote } => {
                self.prefs.set_restart_vote(player.uid, vote).await;
            }
        }
    }

    async fn on_cmd(&self, _from_login: &str, _cmd: PlayerCommand) {
        // update in case we add player commands
    }

    async fn on_admin_cmd(&self, from_login: &str, cmd: AdminCommand<'_>) {
        use AdminCommand::*;
        match cmd {
            Help => {
                let msg = CommandOutput::CommandReference;
                self.widget.show_popup(msg, from_login).await;
            }
            ListMaps => {
                let maps = self.db.maps().await.expect("failed to load maps");
                let msg = CommandOutput::MapList(maps);
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
        };
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
                self.widget
                    .show_popup(CommandOutput::InvalidPlaylistCommand(err), from_login)
                    .await;
            }
        }
    }
}
