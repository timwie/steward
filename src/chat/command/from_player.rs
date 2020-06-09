/// Chat commands for players.
#[derive(Debug)]
pub enum PlayerCommand {
    /// Print a reference of available commands.
    ///
    /// Usage: `/help`
    Help,

    /// Print information about server & controller.
    ///
    /// Usage: `/info`
    Info,
}

impl PlayerCommand {
    /// Parse a player command.
    pub fn from(chat_message: &str) -> Option<PlayerCommand> {
        use PlayerCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/help"] => Some(Help),
            ["/info"] => Some(Info),
            _ => None,
        }
    }
}

/// Player command reference that can be printed in-game.
pub(in crate::chat) const PLAYER_COMMAND_REFERENCE: &str = "
/help     Display this list.
/info     Display information about server & controller.
";
