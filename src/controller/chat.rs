use std::sync::Arc;

use async_trait::async_trait;

use crate::command::{AdminCommand, PlayerCommand};
use crate::controller::LiveSettings;
use crate::event::Command;
use crate::ingame::Server;
use crate::message::{PlayerMessage, ServerMessage};

/// Use to send messages to players.
#[async_trait]
pub trait LiveChat: Send + Sync {
    /// Send a message to the specified player.
    async fn tell(&self, message: PlayerMessage, to_login: &str);
}

#[derive(Clone)]
pub struct ChatController {
    server: Arc<dyn Server>,
    live_settings: Arc<dyn LiveSettings>,
}

impl ChatController {
    pub fn init(server: &Arc<dyn Server>, live_settings: &Arc<dyn LiveSettings>) -> Self {
        ChatController {
            server: server.clone(),
            live_settings: live_settings.clone(),
        }
    }

    /// Forward incoming chat messages:
    /// - if not a `/command`, print the message for all players
    /// - if the `/command` doesn't exist, print the reference for the sender
    /// - if the `/command` is only for admins and the sender isn't one,
    ///   print the reference for the sender
    /// - if proper command, print nothing and return it
    #[allow(clippy::needless_lifetimes)] // TODO how are lifetimes needless here? #1
    pub async fn forward<'a>(&self, message: &'a str, from_login: &str) -> Option<Command<'a>> {
        if let Some(cmd) = PlayerCommand::from(message) {
            return Some(Command::Player {
                cmd,
                from: from_login.to_string(),
            });
        }
        if !self.live_settings.is_admin(&from_login) {
            return None;
        }
        match AdminCommand::from(message) {
            None => {
                // Neither player nor admin command: forward as normal message.
                self.forward_to_all(message, from_login).await;
                None
            }
            Some(cmd) => {
                // Forward other commands to other controllers.
                Some(Command::Admin {
                    cmd,
                    from: from_login.to_string(),
                })
            }
        }
    }

    /// Send a message to all players.
    pub async fn announce(&self, message: ServerMessage<'_>) {
        let message_str = message.to_string();
        if message_str.is_empty() {
            return;
        }
        log::debug!("server msg> {}", &message);
        self.server.chat_send(&message.to_string()).await;
    }

    async fn forward_to_all(&self, message: &str, from_login: &str) {
        if message.is_empty() {
            return;
        }
        self.server
            .chat_send_from_to(message, from_login, vec![])
            .await
            .unwrap();
    }
}

#[async_trait]
impl LiveChat for ChatController {
    async fn tell(&self, message: PlayerMessage, to_login: &str) {
        let message_str = message.to_string();
        if message_str.is_empty() {
            return;
        }
        log::debug!("player msg @{}> {}", &to_login, &message_str);

        // Assume any fault is due to the target player disconnecting.
        let _ = self
            .server
            .chat_send_to(&message.to_string(), vec![to_login])
            .await;
    }
}
