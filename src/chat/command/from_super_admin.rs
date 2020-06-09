/// Chat commands that can only be executed by super admins.
#[derive(Debug)]
pub enum SuperAdminCommand {
    /// Print a reference of available super admin commands.
    ///
    /// Usage: `/help`
    Help,

    /// Prepare a dangerous command, that will only be executed
    /// after taking futher action.
    ///
    /// Usage: see `DangerousCommand`
    Unconfirmed(DangerousCommand),
}

/// Destructive chat commands that can only be executed by super admins,
/// after explicitly confirming them.
#[derive(Debug, Clone)]
pub enum DangerousCommand {
    /// Delete a map that is not in the playlist from the database.
    ///
    /// Usage: `/delete map <uid>`
    DeleteMap { uid: String },

    /// Delete a blacklisted player from the database.
    ///
    /// Usage: `/delete player <login>`
    DeletePlayer { login: String },

    /// Shutdown the server.
    ///
    /// Usage: `/shutdown`
    Shutdown,
}

impl SuperAdminCommand {
    /// Parse a super admin command.
    pub fn from(chat_message: &str) -> Option<SuperAdminCommand> {
        use DangerousCommand::*;
        use SuperAdminCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/delete", "map", uid] => Some(Unconfirmed(DeleteMap {
                uid: (*uid).to_string(),
            })),
            ["/delete", "player", login] => Some(Unconfirmed(DeletePlayer {
                login: (*login).to_string(),
            })),
            ["/help"] => Some(Help),
            ["/shutdown"] => Some(Unconfirmed(Shutdown)),
            _ => None,
        }
    }
}

/// Super admin command reference that can be printed in-game.
pub(in crate::chat) const SUPER_ADMIN_COMMAND_REFERENCE: &str = "
/delete map <uid>         Delete a map from the database. Needs confirmation.
/delete player <login>    Delete a player from the database. Needs confirmation.
/shutdown                 Shutdown the server. Needs confirmation.
";
