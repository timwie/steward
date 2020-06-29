use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::chat::{
    AdminCommand, DangerousCommand, PlayerCommand, PlayerMessage, ServerMessage, SuperAdminCommand,
};
use crate::controller::LiveConfig;
use crate::event::Command;
use crate::server::Server;

/// Use to send messages to players.
#[async_trait]
pub trait LiveChat: Send + Sync {
    /// Send a message to the specified player.
    async fn tell(&self, message: PlayerMessage, to_login: &str);
}

#[derive(Clone)]
pub struct ChatController {
    server: Arc<dyn Server>,
    live_config: Arc<dyn LiveConfig>,
    state: Arc<RwLock<ChatState>>,
}

struct ChatState {
    /// Maps the logins of super admins to dangerous commands that are yet
    /// to be confirmed.
    unconfirmed: HashMap<String, DangerousCommand>,
}

impl ChatState {
    pub fn init() -> Self {
        ChatState {
            unconfirmed: HashMap::new(),
        }
    }
}

impl ChatController {
    pub fn init(server: &Arc<dyn Server>, live_config: &Arc<dyn LiveConfig>) -> Self {
        ChatController {
            server: server.clone(),
            live_config: live_config.clone(),
            state: Arc::new(RwLock::new(ChatState::init())),
        }
    }

    /// If the specified super admin has issued a dangerous command,
    /// this will return it. Subsequent calls would return `None`, until
    /// the admin issues another dangerous command.
    pub async fn pop_unconfirmed_command(&self, from_login: &str) -> Option<DangerousCommand> {
        (*self.state.write().await)
            .unconfirmed
            .remove(&from_login.to_string())
    }

    /// Forward incoming chat messages:
    /// - if not a `/command`, print the message for all players
    /// - if the `/command` doesn't exist, print the reference for the sender
    /// - if the `/command` is only for admins and the sender isn't one,
    ///   print the reference for the sender
    /// - if proper command, print nothing and return it
    /// - dangerous commands in `SuperAdminCommand::Unconfirmed` will be stored,
    ///   and can be retrieved using `pop_unconfirmed_command`, once they have been
    ///   confirmed
    pub async fn forward<'a>(&self, message: &'a str, from_login: &'a str) -> Option<Command<'a>> {
        if !message.starts_with('/') {
            // Neither player nor admin command: forward as normal message.
            self.forward_to_all(message, from_login).await;
            return None;
        }

        // Check if super admin command.
        if self.live_config.is_super_admin(&from_login).await {
            if let Some(cmd) = SuperAdminCommand::from(message) {
                if let SuperAdminCommand::Unconfirmed(dangerous_cmd) = &cmd {
                    (*self.state.write().await)
                        .unconfirmed
                        .insert(from_login.to_string(), dangerous_cmd.clone());
                }
                return Some(Command::SuperAdmin {
                    cmd,
                    from: from_login,
                });
            }
        }

        // Check if admin command.
        if self.live_config.is_admin(&from_login).await {
            if let Some(cmd) = AdminCommand::from(message) {
                return Some(Command::Admin {
                    cmd,
                    from: from_login,
                });
            }
        }

        // Check if player command.
        if let Some(cmd) = PlayerCommand::from(message) {
            return Some(Command::Player {
                cmd,
                from: from_login,
            });
        }

        // Not a known command - return the appropriate ::Help command
        if self.live_config.is_super_admin(&from_login).await {
            Some(Command::SuperAdmin {
                cmd: SuperAdminCommand::Help,
                from: from_login,
            })
        } else if self.live_config.is_admin(&from_login).await {
            Some(Command::Admin {
                cmd: AdminCommand::Help,
                from: from_login,
            })
        } else {
            Some(Command::Player {
                cmd: PlayerCommand::Help,
                from: from_login,
            })
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
            .expect("failed to send server message");
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
