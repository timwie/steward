pub(self) use config::*;
pub use facade::Controller;
pub(self) use player::*;
pub(self) use playlist::*;
pub(self) use preference::*;
pub(self) use queue::*;
pub(self) use race::*;
pub(self) use record::*;
pub(self) use schedule::*;
pub(self) use server_rank::*;
pub(self) use widget::*;

use crate::chat::PlayerMessage;
use crate::server::{Calls, Server};

mod config;
mod facade;
mod player;
mod playlist;
mod preference;
mod queue;
mod race;
mod record;
mod schedule;
mod server_rank;
mod widget;

async fn tell(server: &Server, message: PlayerMessage, to_login: &str) {
    let message_str = message.to_string();
    if message_str.is_empty() {
        return;
    }
    log::debug!("player msg @{}> {}", &to_login, &message_str);

    // Assume any fault is due to the target player disconnecting.
    let _ = server
        .chat_send_to(&message.to_string(), vec![to_login])
        .await;
}
