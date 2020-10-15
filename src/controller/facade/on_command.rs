use semver::Version;
use tokio::time::Duration;

use crate::chat::{
    AdminCommand, CommandConfirmResponse, CommandErrorResponse, CommandOutputResponse,
    CommandResponse, DangerousCommand, InfoResponse, PlayerCommand, PlaylistCommandError,
    ServerMessage, SuperAdminCommand,
};
use crate::constants::BLACKLIST_FILE;
use crate::constants::VERSION;
use crate::controller::facade::announce;
use crate::controller::{Controller, LiveConfig, LivePlayers, LivePlaylist};
use crate::database::{MapQueries, PlayerQueries};
use crate::event::{ControllerEvent, PlaylistDiff};
use crate::network::most_recent_controller_version;
use crate::server::{Calls, ModeCalls, PlayerInfo, RoundBasedModeCalls};

impl Controller {
    pub(super) async fn on_cmd(&self, from: &PlayerInfo, cmd: PlayerCommand) {
        use PlayerCommand::*;

        match cmd {
            Info => {
                let controller = self.clone(); // 'self' with 'static lifetime
                let from_login = from.login.to_string(); // allow data to outlive the current scope
                let _ = tokio::spawn(async move {
                    let private_config = &*controller.config.lock().await;
                    let mode_config = controller.config.mode_config().await;
                    let server_info = controller.server.server_build_info().await;
                    let net_stats = controller.server.server_net_stats().await;

                    let most_recent_controller_version = most_recent_controller_version()
                        .await
                        .unwrap_or_else(|_| Version::new(0, 0, 0));

                    let admin_logins = [
                        &private_config.super_admin_whitelist[..],
                        &private_config.admin_whitelist[..],
                    ]
                    .concat();
                    let admin_logins = admin_logins.iter().map(std::ops::Deref::deref).collect();

                    let admins = controller
                        .db
                        .players(admin_logins)
                        .await
                        .expect("failed to load players");

                    let info = InfoResponse {
                        controller_version: VERSION.clone(),
                        most_recent_controller_version,
                        mode_config,
                        server_info,
                        net_stats,
                        admins,
                    };

                    let msg = CommandResponse::Output(CommandOutputResponse::Info(Box::new(info)));
                    controller.widget.show_popup(msg, &from_login).await;
                });
            }
        }
    }

    pub(super) async fn on_admin_cmd(&self, from: &PlayerInfo, cmd: AdminCommand<'_>) {
        use AdminCommand::*;

        let admin_name = match self.players.display_name(&from.login).await {
            Some(name) => name,
            None => return,
        };
        let admin_name = &admin_name.formatted;

        let try_display_name = |login: String| async move {
            self.db
                .player(&login)
                .await
                .expect("failed to load player")
                .map(|p| p.display_name.formatted)
                .unwrap_or_else(|| login)
        };

        match cmd {
            EditConfig => {
                let curr_cfg = self.config.mode_config().await;
                let curr_cfg = curr_cfg.to_string();
                let msg = CommandResponse::Output(CommandOutputResponse::CurrentConfig {
                    repr: &curr_cfg,
                });
                self.widget.show_popup(msg, &from.login).await;
            }

            ListMaps => {
                let playlist = self.server.playlist().await;

                let maps = self.db.maps(vec![]).await.expect("failed to load maps");

                let in_playlist = maps
                    .iter()
                    .filter(|m1| playlist.iter().any(|m2| m1.uid == m2.uid))
                    .collect();
                let not_in_playlist = maps
                    .iter()
                    .filter(|m1| !playlist.iter().any(|m2| m1.uid == m2.uid))
                    .collect();

                let msg = CommandResponse::Output(CommandOutputResponse::MapList {
                    in_playlist,
                    not_in_playlist,
                });

                self.widget.show_popup(msg, &from.login).await;
            }

            ListPlayers => {
                let players_state = self.players.lock().await;
                let msg = CommandResponse::Output(CommandOutputResponse::PlayerList(
                    players_state.info_all(),
                ));
                self.widget.show_popup(msg, &from.login).await;
            }

            PlaylistAdd { uid } => {
                self.on_playlist_cmd(from, self.playlist.add(&uid).await)
                    .await
            }

            PlaylistRemove { uid } => {
                self.on_playlist_cmd(from, self.playlist.remove(&uid).await)
                    .await
            }

            ImportMap { id } => {
                // Download maps in a separate task.
                let controller = self.clone(); // 'self' with 'static lifetime
                let id = id.to_string(); // allow data to outlive the current scope
                let from = from.clone();
                let _ = tokio::spawn(async move {
                    controller
                        .on_playlist_cmd(&from, controller.playlist.import_map(&id).await)
                        .await;
                });
            }

            SkipCurrentMap => {
                if self.server.end_map().await.is_ok() {
                    announce(
                        &self.server,
                        ServerMessage::CurrentMapSkipped { admin_name },
                    )
                    .await;
                }
            }

            RestartCurrentMap => {
                if let Some(diff) = self.queue.force_restart().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;
                }
                announce(&self.server, ServerMessage::ForceRestart { admin_name }).await;
            }

            ForceQueue { uid } => {
                let playlist_state = self.playlist.lock().await;
                let playlist_index = match playlist_state.index_of(uid) {
                    Some(idx) => idx,
                    None => {
                        let msg = CommandResponse::Error(CommandErrorResponse::UnknownMap);
                        self.widget.show_popup(msg, &from.login).await;
                        return;
                    }
                };
                let map = playlist_state
                    .at_index(playlist_index)
                    .expect("no map at this playlist index");

                if let Some(diff) = self.queue.force_queue(playlist_index).await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;

                    announce(
                        &self.server,
                        ServerMessage::ForceQueued {
                            admin_name,
                            map_name: &map.name.formatted,
                        },
                    )
                    .await;
                }
            }

            BlacklistAdd { login } => {
                if login == from.login {
                    // Do not allow admins to blacklist themselves.
                    return;
                }

                let _ = self.players.remove_player(&login).await;
                let _ = self.server.kick_player(&login, Some("Blacklisted")).await;
                let _ = self.server.blacklist_add(&login).await;
                self.server
                    .blacklist_save(BLACKLIST_FILE)
                    .await
                    .expect("failed to save blacklist file");

                announce(
                    &self.server,
                    ServerMessage::PlayerBlacklisted {
                        admin_name,
                        player_name: &try_display_name(login.to_string()).await,
                    },
                )
                .await;
            }

            BlacklistRemove { login } => {
                let blacklist = self.server.blacklist().await;
                if !blacklist.contains(&login.to_string()) {
                    let msg = CommandResponse::Error(CommandErrorResponse::UnknownBlacklistPlayer);
                    self.widget.show_popup(msg, &from.login).await;
                    return;
                }

                let _ = self.server.blacklist_remove(&login).await;
                self.server
                    .blacklist_save(BLACKLIST_FILE)
                    .await
                    .expect("failed to save blacklist file");

                announce(
                    &self.server,
                    ServerMessage::PlayerUnblacklisted {
                        admin_name,
                        player_name: &try_display_name(login.to_string()).await,
                    },
                )
                .await;
            }

            BlacklistClear => {
                let blacklist = self.server.blacklist().await;

                self.server
                    .blacklist_clear(BLACKLIST_FILE)
                    .await
                    .expect("failed to clear blacklist");

                for login in blacklist {
                    announce(
                        &self.server,
                        ServerMessage::PlayerUnblacklisted {
                            admin_name,
                            player_name: &try_display_name(login.to_string()).await,
                        },
                    )
                    .await;
                }
            }

            TogglePause => {
                let status = self.server.pause_status().await;
                if !status.available {
                    // case 1: cannot pause
                    let msg = CommandResponse::Error(CommandErrorResponse::CannotPause);
                    self.widget.show_popup(msg, &from.login).await;
                } else if status.active {
                    // case 2: unpause now
                    assert!(self.server.pause().await.active);
                    let msg = ServerMessage::MatchPaused { admin_name };
                    announce(&self.server, msg).await;
                } else {
                    // case 3: pause now
                    assert!(!self.server.pause().await.active);
                    let msg = ServerMessage::MatchUnpaused { admin_name };
                    announce(&self.server, msg).await;
                }
            }

            ExtendWarmup { secs } => {
                let status = self.server.warmup_status().await;
                if status.active {
                    self.server.warmup_extend(Duration::from_secs(secs)).await;
                    let msg = ServerMessage::WarmupRoundExtended { admin_name, secs };
                    announce(&self.server, msg).await;
                } else {
                    let msg = CommandResponse::Error(CommandErrorResponse::NotInWarmup);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            SkipWarmup => {
                let status = self.server.warmup_status().await;
                if status.active {
                    self.server.force_end_warmup().await;
                    let msg = ServerMessage::WarmupSkipped { admin_name };
                    announce(&self.server, msg).await;
                } else {
                    let msg = CommandResponse::Error(CommandErrorResponse::NotInWarmup);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }
        };
    }

    pub(super) async fn on_super_admin_cmd(&self, from: &PlayerInfo, cmd: SuperAdminCommand<'_>) {
        use DangerousCommand::*;
        use SuperAdminCommand::*;

        match cmd {
            Prepare(DeleteMap { uid }) => match self.playlist.map(&uid).await {
                Some(map) => {
                    let dcmd = DeleteMap { uid };
                    let msg = CommandResponse::Confirm(
                        dcmd,
                        CommandConfirmResponse::ConfirmMapDeletion {
                            file_name: &map.file_name,
                        },
                    );
                    self.widget.show_popup(msg, &from.login).await;
                }
                None if self
                    .db
                    .map(&uid)
                    .await
                    .expect("failed to load map")
                    .is_some() =>
                {
                    let msg = CommandResponse::Error(CommandErrorResponse::CannotDeletePlaylistMap);
                    self.widget.show_popup(msg, &from.login).await;
                }
                None => {
                    let msg = CommandResponse::Error(CommandErrorResponse::UnknownMap);
                    self.widget.show_popup(msg, &from.login).await;
                }
            },

            Prepare(DeletePlayer { login }) => {
                let blacklist = self.server.blacklist().await;
                if blacklist.contains(&login.to_string()) {
                    let dcmd = DeletePlayer { login };
                    let msg = CommandResponse::Confirm(
                        dcmd,
                        CommandConfirmResponse::ConfirmPlayerDeletion { login: &login },
                    );
                    self.widget.show_popup(msg, &from.login).await;
                } else {
                    let msg =
                        CommandResponse::Error(CommandErrorResponse::CannotDeleteWhitelistedPlayer);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            Prepare(Shutdown) => {
                let msg =
                    CommandResponse::Confirm(Shutdown, CommandConfirmResponse::ConfirmShutdown);
                self.widget.show_popup(msg, &from.login).await;
            }
        }
    }

    pub(super) async fn on_dangerous_cmd(&self, from: &PlayerInfo, cmd: DangerousCommand<'_>) {
        use DangerousCommand::*;

        log::warn!("{}> {:#?}", &from.display_name.plain(), &cmd);

        let admin_name = match self.players.display_name(&from.login).await {
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

                announce(
                    &self.server,
                    ServerMessage::MapDeleted {
                        admin_name,
                        map_name: &map.name.formatted,
                    },
                )
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
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            Shutdown => {
                self.server.shutdown_server().await;
            }
        }
    }

    async fn on_playlist_cmd(
        &self,
        from: &PlayerInfo,
        cmd_res: Result<PlaylistDiff, PlaylistCommandError>,
    ) {
        match cmd_res {
            Ok(change) => {
                let ev = ControllerEvent::NewPlaylist(change);
                self.on_controller_event(ev).await;
            }
            Err(err) => {
                let msg = CommandResponse::Error(CommandErrorResponse::InvalidPlaylistCommand(err));
                self.widget.show_popup(msg, &from.login).await;
            }
        }
    }
}
