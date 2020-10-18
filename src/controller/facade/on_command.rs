use semver::Version;
use tokio::time::Duration;

use crate::chat::{
    AdminCommand, CommandConfirmOutput, CommandErrorOutput, CommandOutput, CommandResultOutput,
    DangerousCommand, PlayerCommand, PlaylistCommandError, ServerMessage, SuperAdminCommand,
};
use crate::constants::BLACKLIST_FILE;
use crate::constants::VERSION;
use crate::controller::facade::announce;
use crate::controller::{Controller, LiveConfig, LivePlayers, LivePlaylist};
use crate::database::{Map, MapQueries, PlayerQueries};
use crate::event::{ControllerEvent, PlaylistDiff};
use crate::network::most_recent_controller_version;
use crate::server::{Calls, ModeCalls, ModeScript, PlayerInfo, RoundBasedModeCalls};

impl Controller {
    pub(super) async fn on_cmd(&self, from: &PlayerInfo, cmd: PlayerCommand) {
        use CommandOutput::*;
        use CommandResultOutput::*;

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

                    let info = crate::chat::ControllerInfo {
                        controller_version: VERSION.clone(),
                        most_recent_controller_version,
                        mode_config,
                        server_info,
                        net_stats,
                        admins,
                    };

                    let msg = Result(ControllerInfo(Box::new(info)));
                    controller.widget.show_popup(msg, &from_login).await;
                });
            }
        }
    }

    pub(super) async fn on_admin_cmd(&self, from: &PlayerInfo, cmd: AdminCommand<'_>) {
        use CommandErrorOutput::*;
        use CommandOutput::*;
        use CommandResultOutput::*;

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
                let msg = Result(CurrentConfig { repr: &curr_cfg });
                self.widget.show_popup(msg, &from.login).await;
            }

            ListMaps => {
                let playlist = self.server.playlist().await;

                let maps = self.db.maps(vec![]).await.expect("failed to load maps");

                let mut in_playlist: Vec<&Map> = maps
                    .iter()
                    .filter(|m1| playlist.iter().any(|m2| m1.uid == m2.uid))
                    .collect();
                let mut not_in_playlist: Vec<&Map> = maps
                    .iter()
                    .filter(|m1| !playlist.iter().any(|m2| m1.uid == m2.uid))
                    .collect();

                in_playlist.sort_by_key(|map| map.name.plain());
                not_in_playlist.sort_by_key(|map| map.name.plain());

                let msg = Result(MapList {
                    in_playlist,
                    not_in_playlist,
                });

                self.widget.show_popup(msg, &from.login).await;
            }

            ListPlayers => {
                let players_state = self.players.lock().await;
                let msg = Result(PlayerList(players_state.info_all()));
                self.widget.show_popup(msg, &from.login).await;
            }

            PlaylistAdd { uid } => {
                self.on_playlist_cmd(from, self.playlist.add(&uid).await)
                    .await
            }

            PlaylistAddAll => {
                let maps = self.db.maps(vec![]).await.expect("failed to load maps");

                for map in maps {
                    if self.playlist.index_of(&map.uid).await.is_some() {
                        continue;
                    }

                    self.on_playlist_cmd(from, self.playlist.add(&map.uid).await)
                        .await;
                }
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
                        let msg = Error(UnknownMap);
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
                    let msg = Error(UnknownBlacklistPlayer);
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
                    let msg = Error(CannotPause);
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
                    let msg = Error(NotInWarmup);
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
                    let msg = Error(NotInWarmup);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            KickPlayer {
                login_or_display_name,
            } => {
                let players_state = self.players.lock().await;

                let maybe_player = players_state
                    .info(&login_or_display_name)
                    .or_else(|| players_state.display_name_info(&login_or_display_name));

                let player = match maybe_player {
                    Some(player) => player,
                    None => {
                        let msg = Error(UnknownPlayer);
                        self.widget.show_popup(msg, &from.login).await;
                        return;
                    }
                };

                if self.server.kick_player(&player.login, None).await.is_ok() {
                    let msg = ServerMessage::PlayerKicked {
                        admin_name,
                        player_name: &player.display_name.formatted,
                    };
                    announce(&self.server, msg).await;
                } else {
                    let msg = Error(UnknownPlayer);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            MovePlayerToSpectator {
                login_or_display_name,
            } => {
                let players_state = self.players.lock().await;

                let maybe_player = players_state
                    .info(&login_or_display_name)
                    .or_else(|| players_state.display_name_info(&login_or_display_name));

                let player = match maybe_player {
                    Some(player) => player,
                    None => {
                        let msg = Error(UnknownPlayer);
                        self.widget.show_popup(msg, &from.login).await;
                        return;
                    }
                };

                // TODO this might not free up their player slot!
                if self.server.force_spectator(&player.login).await.is_ok() {
                    let msg = ServerMessage::PlayerMovedToSpectator {
                        admin_name,
                        player_name: &player.display_name.formatted,
                    };
                    announce(&self.server, msg).await;
                } else {
                    let msg = Error(UnknownPlayer);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            ChangeMode { script_name } => {
                let maybe_default_mode = ModeScript::default_modes()
                    .into_iter()
                    .find(|mode| mode.name().to_lowercase() == script_name.to_lowercase());

                match maybe_default_mode {
                    None => {
                        let msg = Error(UnknownMode {
                            tried: script_name,
                            options: ModeScript::default_modes(),
                        });
                        self.widget.show_popup(msg, &from.login).await;
                    }
                    Some(mode) => match self.server.set_mode(mode.clone()).await {
                        Ok(_) => {
                            announce(
                                &self.server,
                                ServerMessage::ModeChanging { admin_name, mode },
                            )
                            .await;
                        }
                        Err(fault) => {
                            let msg = Error(CannotChangeMode { msg: &fault.msg });
                            self.widget.show_popup(msg, &from.login).await;
                        }
                    },
                }
            }

            LoadSettings { file_name } => {
                let file_name = format!("{}.txt", file_name.trim_end_matches(".txt"));

                match self.server.load_match_settings(&file_name).await {
                    Ok(_) => {
                        announce(
                            &self.server,
                            ServerMessage::LoadedMatchSettings {
                                admin_name,
                                settings_name: &file_name.trim_end_matches(".txt"),
                            },
                        )
                        .await;
                    }
                    Err(_) => {
                        let dir = self
                            .server
                            .user_data_dir()
                            .await
                            .join("Maps")
                            .join("MatchSettings");
                        let paths =
                            std::fs::read_dir(dir).expect("failed to list match settings files");

                        let mut options: Vec<String> = paths
                            .filter_map(|entry| {
                                let entry = entry.expect("failed to list match settings files");
                                match entry.file_name().to_str() {
                                    Some(name) if name.ends_with(".txt") => Some(name.to_string()),
                                    _ => None,
                                }
                            })
                            .collect();
                        options.sort();

                        let msg = Error(UnknownMatchSettings {
                            tried: &file_name,
                            options,
                        });
                        self.widget.show_popup(msg, &from.login).await;
                    }
                }
            }

            SaveSettings { file_name } => {
                let file_name = format!("{}.txt", file_name.trim_end_matches(".txt"));

                match self.server.save_match_settings(&file_name).await {
                    Ok(_) => {
                        announce(
                            &self.server,
                            ServerMessage::SavedMatchSettings {
                                admin_name,
                                settings_name: &file_name.trim_end_matches(".txt"),
                            },
                        )
                        .await;
                    }
                    Err(fault) => {
                        let msg = Error(CannotSaveMatchSettings { msg: &fault.msg });
                        self.widget.show_popup(msg, &from.login).await;
                    }
                }
            }
        };
    }

    pub(super) async fn on_super_admin_cmd(&self, from: &PlayerInfo, cmd: SuperAdminCommand<'_>) {
        use CommandConfirmOutput::*;
        use CommandErrorOutput::*;
        use CommandOutput::*;

        use DangerousCommand::*;
        use SuperAdminCommand::*;

        match cmd {
            Prepare(DeleteMap { uid }) => {
                match self.db.map(&uid).await.expect("failed to load map") {
                    Some(map) => {
                        if self.playlist.index_of(&uid).await.is_some() {
                            let msg = Error(CannotDeletePlaylistMap);
                            self.widget.show_popup(msg, &from.login).await;
                            return;
                        }

                        let dcmd = DeleteMap { uid };
                        let msg = Confirm(dcmd, ConfirmMapDeletion { map: &map });
                        self.widget.show_popup(msg, &from.login).await;
                    }
                    None => {
                        let msg = Error(UnknownMap);
                        self.widget.show_popup(msg, &from.login).await;
                    }
                }
            }

            Prepare(DeletePlayer { login }) => {
                let blacklist = self.server.blacklist().await;
                if blacklist.contains(&login.to_string()) {
                    let dcmd = DeletePlayer { login };
                    let msg = Confirm(dcmd, ConfirmPlayerDeletion { login: &login });
                    self.widget.show_popup(msg, &from.login).await;
                } else {
                    let msg = Error(CannotDeleteWhitelistedPlayer);
                    self.widget.show_popup(msg, &from.login).await;
                }
            }

            Prepare(Shutdown) => {
                let msg = Confirm(Shutdown, ConfirmShutdown);
                self.widget.show_popup(msg, &from.login).await;
            }
        }
    }

    pub(super) async fn on_dangerous_cmd(&self, from: &PlayerInfo, cmd: DangerousCommand<'_>) {
        use CommandErrorOutput::*;
        use CommandOutput::*;

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
                if let Some(file_name) = map.file_name {
                    let map_path = self.config.maps_dir().await.join(file_name);
                    if map_path.is_file() {
                        std::fs::remove_file(map_path).expect("failed to delete map file");
                    }
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
                    let msg = Error(UnknownPlayer);
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
        use CommandErrorOutput::*;
        use CommandOutput::*;

        match cmd_res {
            Ok(change) => {
                let ev = ControllerEvent::NewPlaylist(change);
                self.on_controller_event(ev).await;
            }
            Err(err) => {
                let msg = Error(InvalidPlaylistCommand(err));
                self.widget.show_popup(msg, &from.login).await;
            }
        }
    }
}
