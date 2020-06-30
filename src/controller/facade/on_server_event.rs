use crate::controller::{Controller, LiveConfig, LivePlaylist};
use crate::event::ControllerEvent;
use crate::server::ServerEvent;
use crate::widget::Action;

impl Controller {
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
                    let ev = ControllerEvent::BeginMap(loaded_map);
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
                let outro_ev = ControllerEvent::BeginOutro;
                self.on_controller_event(outro_ev).await;

                // Delay for the duration of the vote.
                // Spawn a task to not block the callback loop.
                let controller = self.clone(); // 'self' with 'static lifetime
                let vote_duration = self.config.vote_duration().await;
                let _ = tokio::spawn(async move {
                    log::debug!("start vote");
                    tokio::time::delay_for(
                        vote_duration.to_std().expect("failed to delay vote end"),
                    )
                    .await;
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
                            let ev = ControllerEvent::EndRun(pb_diff);
                            controller.on_controller_event(ev).await;
                        }
                    });
                }
            }

            ServerEvent::PlayerAnswered {
                from_login, answer, ..
            } => {
                let action = Action::from_answer(answer);
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
                if let Some(cmd) = self.chat.forward(&message, &from_login).await {
                    self.on_controller_event(ControllerEvent::IssueCommand(cmd))
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
}
