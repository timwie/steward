use std::default::Default;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::chat::{BadCommandContext, CommandContext, CommandEnum, CommandReference, PlayerRole};

/// Chat commands that can only be executed by super admins.
#[derive(Debug, Clone, Copy)]
pub enum SuperAdminCommand<'a> {
    /// Prepare a dangerous command, that will only be executed
    /// after taking futher action.
    ///
    /// Usage: see `DangerousCommand`
    Prepare(DangerousCommand<'a>),
}

/// Destructive chat commands that can only be executed by super admins,
/// after explicitly confirming them.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DangerousCommand<'a> {
    /// Delete a map that is not in the playlist from the database.
    ///
    /// Usage: `/delete map <uid>`
    DeleteMap { uid: &'a str },

    /// Delete a blacklisted player from the database.
    ///
    /// Usage: `/delete player <login>`
    DeletePlayer { login: &'a str },

    /// Shutdown the server.
    ///
    /// Usage: `/shutdown`
    Shutdown,
}

lazy_static! {
    static ref SUPER_ADMIN_COMMANDS: Vec<SuperAdminCommand<'static>> = {
        use DangerousCommand::*;
        use SuperAdminCommand::*;
        vec![
            Prepare(DeleteMap {
                uid: Default::default(),
            }),
            Prepare(DeletePlayer {
                login: Default::default(),
            }),
            Prepare(Shutdown),
        ]
    };
}

impl<'a> CommandEnum<'a> for SuperAdminCommand<'a> {
    fn all() -> &'static Vec<Self> {
        &SUPER_ADMIN_COMMANDS
    }

    fn parse(chat_message: &'a str) -> Option<Self> {
        use DangerousCommand::*;
        use SuperAdminCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/delete", "map", uid] => Some(Prepare(DeleteMap { uid })),
            ["/delete", "player", login] => Some(Prepare(DeletePlayer { login })),
            ["/shutdown"] => Some(Prepare(Shutdown)),
            _ => None,
        }
    }

    fn check(&self, ctxt: CommandContext) -> Result<(), BadCommandContext> {
        use BadCommandContext::*;

        match self {
            _ if ctxt.player_role < PlayerRole::SuperAdmin => Err(NoPermission),
            _ => Ok(()),
        }
    }

    fn reference(&self) -> CommandReference {
        use DangerousCommand::*;
        use SuperAdminCommand::*;

        match self {
            Prepare(DeleteMap { .. }) => {
                ("/delete map <uid>", "Delete a map from the database").into()
            }
            Prepare(DeletePlayer { .. }) => (
                "/delete player <login>",
                "Delete a player from the database",
            )
                .into(),
            Prepare(Shutdown) => ("/shutdown", "Shutdown the server.").into(),
        }
    }
}
