use std::str::FromStr;

use futures::future::join_all;

use crate::chat::{CommandOutputResponse, CommandResponse};
use crate::config::TimeAttackConfig;
use crate::controller::{ActivePreference, Controller};
use crate::event::ControllerEvent;
use crate::server::PlayerInfo;
use crate::widget::Action;

impl Controller {
    pub(super) async fn on_action(&self, player: &PlayerInfo, action: Action) {
        use Action::*;

        match action {
            SetConfig { repr } => match TimeAttackConfig::from_str(&repr) {
                Ok(new_cfg) => {
                    let changes = self.config.set_mode_config(new_cfg).await;
                    join_all(changes.into_iter().map(|change| async move {
                        let ev = ControllerEvent::NewConfig {
                            change,
                            from_login: &player.login,
                        };
                        self.on_controller_event(ev).await;
                    }))
                    .await;
                }
                Err(de_err) => {
                    let err_msg = format!("{:#?}", de_err);
                    let msg = CommandResponse::Output(CommandOutputResponse::InvalidConfig {
                        tried_repr: &repr,
                        error_msg: &err_msg,
                    });
                    self.widget.show_popup(msg, &player.login).await;
                }
            },

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
                let pref = ActivePreference {
                    map_uid: map_uid.to_string(),
                    player_uid: player.uid,
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
}
