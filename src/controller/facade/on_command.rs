use semver::Version;
use tokio::time::Duration;

use crate::chat::{
    AdminCommand, CommandConfirmResponse, CommandErrorResponse, CommandOutputResponse,
    CommandResponse, DangerousCommand, PlayerCommand, PlaylistCommandError, ServerMessage,
    SuperAdminCommand,
};
use crate::constants::BLACKLIST_FILE;
use crate::constants::VERSION;
use crate::controller::{Controller, LiveConfig, LivePlayers, LivePlaylist};
use crate::event::{ControllerEvent, PlaylistDiff};
use crate::network::most_recent_controller_version;

impl Controller {
    pub(super) async fn on_cmd(&self, from_login: &str, cmd: PlayerCommand) {
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
                    let private_config = &*controller.config.lock().await;
                    let public_config = &controller.config.public_config().await;
                    let server_info = &controller.server.server_info().await;
                    let net_stats = &controller.server.net_stats().await;
                    let blacklist = &controller.server.blacklist().await;
                    let msg = CommandResponse::Output(CommandOutputResponse::Info {
                        controller_version: &VERSION,
                        most_recent_controller_version,
                        private_config,
                        public_config,
                        server_info,
                        net_stats,
                        blacklist,
                    });
                    controller.widget.show_popup(msg, &from_login).await;
                });
            }
        }
    }

    pub(super) async fn on_admin_cmd(&self, from_login: &str, cmd: AdminCommand<'_>) {
        use AdminCommand::*;

        let admin_name = match self.players.nick_name(from_login).await {
            Some(name) => name,
            None => return,
        };
        let admin_name = &admin_name.formatted;

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

            EditConfig => {
                let curr_cfg = self.config.public_config().await;
                let curr_cfg = curr_cfg.write();
                let msg = CommandResponse::Output(CommandOutputResponse::CurrentConfig {
                    repr: &curr_cfg,
                });
                self.widget.show_popup(msg, from_login).await;
            }

            ListMaps => {
                let maps = self.db.maps().await.expect("failed to load maps");
                let msg =
                    CommandResponse::Output(CommandOutputResponse::MapList(maps.iter().collect()));
                self.widget.show_popup(msg, from_login).await;
            }

            ListPlayers => {
                let players_state = self.players.lock().await;
                let msg = CommandResponse::Output(CommandOutputResponse::PlayerList(
                    players_state.info_all(),
                ));
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
                    .announce(ServerMessage::CurrentMapSkipped { admin_name })
                    .await;
            }

            RestartCurrentMap => {
                if let Some(diff) = self.queue.force_restart().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;

                    self.chat
                        .announce(ServerMessage::ForceRestart { admin_name })
                        .await;
                }
            }

            ForceQueue { uid } => {
                let playlist_state = self.playlist.lock().await;
                let playlist_index = match playlist_state.index_of(uid) {
                    Some(idx) => idx,
                    None => {
                        let msg = CommandResponse::Error(CommandErrorResponse::UnknownMap);
                        self.widget.show_popup(msg, from_login).await;
                        return;
                    }
                };
                let map = playlist_state
                    .at_index(playlist_index)
                    .expect("no map at this playlist index");

                if let Some(diff) = self.queue.force_queue(playlist_index).await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;

                    self.chat
                        .announce(ServerMessage::ForceQueued {
                            admin_name,
                            map_name: &map.name.formatted,
                        })
                        .await;
                }
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
                        admin_name,
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
                        admin_name,
                        player_name: &or_nickname(login.to_string()).await,
                    })
                    .await;
            }

            TogglePause => {
                let status = self.server.pause_status().await;
                if !status.available {
                    // case 1: cannot pause
                    let msg = CommandResponse::Error(CommandErrorResponse::CannotPause);
                    self.widget.show_popup(msg, from_login).await;
                } else if status.active {
                    // case 2: unpause now
                    assert!(self.server.pause().await.active);
                    let msg = ServerMessage::MatchPaused { admin_name };
                    self.chat.announce(msg).await;
                } else {
                    // case 3: pause now
                    assert!(!self.server.pause().await.active);
                    let msg = ServerMessage::MatchUnpaused { admin_name };
                    self.chat.announce(msg).await;
                }
            }

            ExtendWarmup { secs } => {
                let status = self.server.warmup_status().await;
                if status.active {
                    self.server.warmup_extend(Duration::from_secs(secs)).await;
                    let msg = ServerMessage::WarmupRoundExtended { admin_name, secs };
                    self.chat.announce(msg).await;
                } else {
                    let msg = CommandResponse::Error(CommandErrorResponse::NotInWarmup);
                    self.widget.show_popup(msg, from_login).await;
                }
            }

            SkipWarmup => {
                let status = self.server.warmup_status().await;
                if status.active {
                    self.server.force_end_warmup().await;
                    let msg = ServerMessage::WarmupSkipped { admin_name };
                    self.chat.announce(msg).await;
                } else {
                    let msg = CommandResponse::Error(CommandErrorResponse::NotInWarmup);
                    self.widget.show_popup(msg, from_login).await;
                }
            }
        };
    }

    pub(super) async fn on_super_admin_cmd(&self, from_login: &str, cmd: SuperAdminCommand) {
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

    pub(super) async fn on_dangerous_cmd(&self, from_login: &str, cmd: DangerousCommand) {
        use DangerousCommand::*;

        log::warn!("{}> {:#?}", from_login, &cmd);

        let admin_name = match self.players.nick_name(from_login).await {
            Some(name) => name,
            None => return,
        };
        let admin_name = &admin_name.formatted;

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
                let map_path = self.config.maps_dir().await.join(map.file_name);
                if map_path.is_file() {
                    std::fs::remove_file(map_path).expect("failed to delete map file");
                }

                self.chat
                    .announce(ServerMessage::MapDeleted {
                        admin_name,
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
