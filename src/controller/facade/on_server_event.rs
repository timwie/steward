use crate::controller::Controller;
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
                if is_restart {
                    let ev = ControllerEvent::EndOutro;
                    self.on_controller_event(ev).await;
                }

                let ev = ControllerEvent::BeginIntro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::MapUnload => {
                let ev = ControllerEvent::ChangeMap;
                self.on_controller_event(ev).await;

                let ev = ControllerEvent::EndOutro;
                self.on_controller_event(ev).await;
            }

            ServerEvent::RaceEnd => {
                let outro_ev = ControllerEvent::BeginOutro;
                self.on_controller_event(outro_ev).await;
            }

            ServerEvent::RunCountdown { player_login } => {
                const COUNTDOWN_SECS: u64 = 2;

                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    tokio::time::delay_for(tokio::time::Duration::from_secs(COUNTDOWN_SECS)).await;
                    let ev = ControllerEvent::BeginRun {
                        player_login: &player_login,
                    };
                    controller.on_controller_event(ev).await;
                });
            }

            ServerEvent::RunStartline { player_login } => {
                let ev = ControllerEvent::BeginRun {
                    player_login: &player_login,
                };
                self.on_controller_event(ev).await;
            }

            ServerEvent::RunCheckpoint { event } => {
                let ev = ControllerEvent::ContinueRun(event);
                self.on_controller_event(ev).await;
            }

            ServerEvent::RunIncoherence { player_login } => {
                let ev = ControllerEvent::DesyncRun {
                    player_login: &player_login,
                };
                self.on_controller_event(ev).await;
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
        }
    }
}
