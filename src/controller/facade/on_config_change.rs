use crate::chat::ServerMessage;
use crate::controller::facade::announce;
use crate::controller::{Controller, LivePlayers};
use crate::event::ConfigDiff;

impl Controller {
    pub(super) async fn on_config_change(&self, from_login: &str, diff: ConfigDiff) {
        use ConfigDiff::*;

        let from_display_name = match self.players.display_name(from_login).await {
            Some(name) => name,
            None => return,
        };

        match diff {
            NewTimeLimit { .. } => {
                self.schedule.set_time_limit().await;
                self.widget.refresh_schedule().await;

                announce(
                    &self.server,
                    ServerMessage::TimeLimitChanged {
                        admin_name: &from_display_name.formatted,
                    },
                )
                .await;
            }
            NewOutroDuration { .. } => {
                self.widget.refresh_schedule().await;
            }
        }
    }
}
