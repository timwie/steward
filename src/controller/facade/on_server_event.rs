use crate::chat::{Command, CommandContext, CommandErrorResponse, CommandResponse};
use crate::controller::{Controller, LiveConfig, LivePlayers};
use crate::event::ControllerEvent;
use crate::server::{Calls, ModeScriptSection, ServerEvent};
use crate::widget::Action;

impl Controller {
    /// Server events are converted to controller events with the
    /// help of one or more controllers.
    pub async fn on_server_event(&self, event: ServerEvent) {
        use ModeScriptSection::*;

        log::debug!("{:#?}", &event);
        match event {
            ServerEvent::PlayerInfoChanged(info) => {
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

            ServerEvent::PlayerCountdown { login } => {
                const COUNTDOWN_SECS: u64 = 2;

                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    tokio::time::delay_for(tokio::time::Duration::from_secs(COUNTDOWN_SECS)).await;
                    let ev = ControllerEvent::BeginRun {
                        player_login: &login,
                    };
                    controller.on_controller_event(ev).await;
                });
            }

            ServerEvent::PlayerStartline { login } => {
                let ev = ControllerEvent::BeginRun {
                    player_login: &login,
                };
                self.on_controller_event(ev).await;
            }

            ServerEvent::PlayerCheckpoint(event) => {
                let ev = ControllerEvent::ContinueRun(event);
                self.on_controller_event(ev).await;
            }

            ServerEvent::PlayerIncoherence { login } => {
                let ev = ControllerEvent::DesyncRun {
                    player_login: &login,
                };
                self.on_controller_event(ev).await;
            }

            ServerEvent::PlayerAnswered {
                from_login,
                mut answer,
                ..
            } => {
                let action = Action::from_answer(&mut answer);
                let ev = ControllerEvent::IssueAction {
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
                // FIXME this is only PoC
                //  => build the context from state
                let cfg = self.config.lock().await;
                let player = self.players.info(&from_login).await.unwrap();
                let mode = self.server.mode().await.script;
                let warmup = self.server.warmup_status().await;
                let pause = self.server.pause_status().await;

                let player_role = cfg.role_of(&from_login);

                let ctxt = CommandContext {
                    cmd: &message,
                    player: &player,
                    mode: &mode,
                    player_role,
                    warmup: &warmup,
                    pause: &pause,
                };

                if !message.starts_with('/') {
                    // Message is not a command
                    let ev = ControllerEvent::ChatMessage {
                        from: &player,
                        message: &message,
                    };
                    self.on_controller_event(ev).await;
                    return;
                }

                match Command::try_from(ctxt) {
                    Ok(cmd) => {
                        let ev = ControllerEvent::IssueCommand(ctxt, cmd);
                        self.on_controller_event(ev).await;
                    }
                    Err(err) => {
                        let msg =
                            CommandResponse::Error(CommandErrorResponse::CommandError(ctxt, err));
                        self.widget.show_popup(msg, &player.login).await;
                    }
                }
            }

            ServerEvent::Scores(scores) => {
                // This event is only useful when triggering it to get the score
                // at controller start. Otherwise, we can update it whenever
                // a player finishes a run.
                self.race.set_scores(&scores).await;
            }

            ServerEvent::PauseStatus(status) => {
                if self.race.set_pause(status.active).await {
                    let ev = if status.active {
                        ControllerEvent::BeginPause
                    } else {
                        ControllerEvent::EndPause
                    };
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::WarmupStatus(status) => {
                if self.race.set_warmup(status.active).await {
                    let ev = if status.active {
                        ControllerEvent::BeginWarmup
                    } else {
                        ControllerEvent::EndWarmup
                    };
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::WarmupBegin(_) => {
                let ev = ControllerEvent::BeginWarmup;
                self.on_controller_event(ev).await;
            }

            ServerEvent::WarmupEnd(status) => {
                if status.current_round == status.nb_total_rounds {
                    let ev = ControllerEvent::EndWarmup;
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::ModeScriptSection(PreStartServer {
                restarted_script,
                changed_script,
            }) => {
                if restarted_script || changed_script {
                    let mode_options = self.server.mode_options().await;
                    let mode_script = mode_options.script();

                    let ev = ControllerEvent::ChangeMode(mode_script);
                    self.on_controller_event(ev).await;
                }
            }

            ServerEvent::ModeScriptSection(PostStartServer) => {}

            ServerEvent::ModeScriptSection(PreStartMatch) => {}
            ServerEvent::ModeScriptSection(PostStartMatch) => {}

            ServerEvent::ModeScriptSection(PreLoadMap { is_restart }) => {
                if is_restart {
                    let ev = ControllerEvent::EndOutro;
                    self.on_controller_event(ev).await;
                }

                let ev = ControllerEvent::BeginIntro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::ModeScriptSection(PostLoadMap) => {}

            ServerEvent::ModeScriptSection(PreStartMap) => {}
            ServerEvent::ModeScriptSection(PostStartMap) => {}

            ServerEvent::ModeScriptSection(PreStartRound) => {}
            ServerEvent::ModeScriptSection(PostStartRound) => {}

            ServerEvent::ModeScriptSection(PrePlayloop) => {}
            ServerEvent::ModeScriptSection(PostPlayloop) => {}

            ServerEvent::ModeScriptSection(PreEndRound) => {}
            ServerEvent::ModeScriptSection(PostEndRound) => {}

            ServerEvent::ModeScriptSection(PreEndMap) => {}
            ServerEvent::ModeScriptSection(PostEndMap) => {}

            ServerEvent::ModeScriptSection(PreUnloadMap) => {
                let ev = ControllerEvent::ChangeMap;
                self.on_controller_event(ev).await;

                let ev = ControllerEvent::EndOutro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::ModeScriptSection(PostUnloadMap) => {}

            ServerEvent::ModeScriptSection(PreEndMatch) => {}

            ServerEvent::ModeScriptSection(PostEndMatch) => {
                let outro_ev = ControllerEvent::BeginOutro;
                self.on_controller_event(outro_ev).await;
            }

            ServerEvent::ModeScriptSection(PreEndServer) => {}
            ServerEvent::ModeScriptSection(PostEndServer) => {}

            ServerEvent::PlaylistChanged { curr_idx, .. } => {
                // TODO sync playlist
                if let Some(curr_idx) = curr_idx {
                    self.playlist.set_index(curr_idx as usize).await;
                }
            }

            ServerEvent::PlayerCheckpointRespawn(_) => {}
            ServerEvent::ChampionRoundEnd(_) => {}

            ServerEvent::KnockoutRoundEnd(_) => {
                // TODO knockout elimination event
                //  => for some reason, eliminated players are listed with their account id only
                //  => that id is not part of PlayerInfo
                //  => instead, the best course of action is probably to request Scores
                //     at the start of a round, since that includes the account id for players
            }
        }
    }
}
