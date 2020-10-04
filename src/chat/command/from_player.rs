use lazy_static::lazy_static;

use crate::chat::{BadCommandContext, CommandContext, CommandEnum, CommandReference};

/// Chat commands for all players.
#[derive(Debug, Copy, Clone)]
pub enum PlayerCommand {
    /// Print information about server & controller.
    ///
    /// Usage: `/info`
    Info,
}

lazy_static! {
    static ref PLAYER_COMMANDS: Vec<PlayerCommand> = {
        use PlayerCommand::*;
        vec![Info]
    };
}

impl CommandEnum<'_> for PlayerCommand {
    fn all() -> &'static Vec<Self> {
        &PLAYER_COMMANDS
    }

    fn parse(chat_message: &str) -> Option<Self> {
        use PlayerCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/info"] => Some(Info),
            _ => None,
        }
    }

    fn check(&self, _ctxt: CommandContext) -> Result<(), BadCommandContext> {
        Ok(())
    }

    fn reference(&self) -> CommandReference {
        use PlayerCommand::*;
        match self {
            Info => ("/info", "Display server & controller information").into(),
        }
    }
}
