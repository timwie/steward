use std::collections::HashMap;

pub use from_admin::*;
pub use from_player::*;
pub use from_super_admin::*;
pub use response::*;

use crate::server::{ModeScript, PauseStatus, PlayerInfo, WarmupStatus};

mod from_admin;
mod from_player;
mod from_super_admin;
mod response;

/// A parsed and validated command that should be executed.
#[derive(Debug, Copy, Clone)]
pub enum Command<'a> {
    Help,
    Player(PlayerCommand),
    Admin(AdminCommand<'a>),
    SuperAdmin(SuperAdminCommand<'a>),
}

/// Reasons why a command should not/cannot be considered for execution.
pub enum CommandDeniedError {
    /// There is no such command.
    NoSuchCommand,

    /// The command cannot be executed in this context.
    NotAvailable(BadCommandContext),
}

/// An attempt of a player to execute a command.
///
/// Whether or not a command is valid depends on the player, mode, section, etc.
/// This struct bundles all information needed to decide whether or not to
/// execute a command (and if not; why?).
#[derive(Debug, Copy, Clone)]
pub struct CommandContext<'a> {
    pub cmd: &'a str,
    pub player: &'a PlayerInfo,
    pub player_role: PlayerRole,
    pub mode: &'a ModeScript,
    pub warmup: &'a WarmupStatus,
    pub pause: &'a PauseStatus,
}

/// Player permission level.
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd)]
pub enum PlayerRole {
    Player,
    Admin,
    SuperAdmin,
}

/// Details why a command cannot be executed in this context.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone)]
pub enum BadCommandContext {
    /// Player can only execute this command during a warmup.
    DuringWarmup,

    /// Player can only execute this command in a specific game mode.
    InMode(ModeScript),

    /// Player can only execute this command in another game mode.
    InOtherModes,

    /// Player does not have the rights to execute this command.
    NoPermission,
}

/// A tuple of a command's example usage and explanation.
#[derive(Debug)]
pub(in crate::chat) struct CommandReference<'a> {
    usage: &'a str,
    doc: &'a str,
}

impl<'a> From<(&'a str, &'a str)> for CommandReference<'a> {
    fn from(tuple: (&'a str, &'a str)) -> Self {
        CommandReference {
            usage: tuple.0,
            doc: tuple.1,
        }
    }
}

/// Commands can be grouped in arbitrary sets, to not have one huge `enum` for all commands.
pub(in crate::chat) trait CommandEnum<'a>: Sized {
    /// All possible command variants in this set.
    ///
    /// This list is just there to build a reference list.
    /// Command parameters should just be dummy/default values.
    fn all() -> &'static Vec<Self>;

    /// Check if a chat message matches any command variant.
    fn parse(chat_message: &'a str) -> Option<Self>;

    /// Check if this command variant can be executed in the given context.
    fn check(&self, ctxt: CommandContext) -> Result<(), BadCommandContext>;

    /// Return the reference of this command variant.
    fn reference(&self) -> CommandReference;
}

impl Command<'_> {
    /// Parse a command in the given context.
    ///
    /// Returns an error variant if the command is unknown, was provided
    /// wrong arguments, or when it cannot be executed in this context.
    pub fn try_from(ctxt: CommandContext) -> Result<Command, CommandDeniedError> {
        if ctxt.cmd.trim() == "/help" {
            return Ok(Command::Help);
        }

        macro_rules! try_command_set {
            ($cmd_type:ty, $cmd_variant:ident) => {
                if let Some(cmd) = <$cmd_type>::parse(ctxt.cmd) {
                    let _ = cmd.check(ctxt).map_err(CommandDeniedError::NotAvailable)?;
                    return Ok(Command::$cmd_variant(cmd));
                }
            };
        }

        try_command_set!(PlayerCommand, Player);
        try_command_set!(AdminCommand, Admin);
        try_command_set!(SuperAdminCommand, SuperAdmin);

        Err(CommandDeniedError::NoSuchCommand)
    }
}

impl CommandContext<'_> {
    /// The command reference for the given context, in tabular form.
    pub(in crate::chat) fn reference(&self) -> String {
        use prettytable::*;
        use BadCommandContext::*;

        let mut cmds = HashMap::<Option<BadCommandContext>, Vec<CommandReference>>::new();

        macro_rules! add_command_set {
            ($cmd_type:ty) => {
                for cmd in <$cmd_type>::all().iter() {
                    cmds.entry(cmd.check(*self).err())
                        .or_insert_with(Vec::new)
                        .push(cmd.reference());
                }
            };
        }

        add_command_set!(PlayerCommand);
        add_command_set!(AdminCommand);
        add_command_set!(SuperAdminCommand);

        let mut sections: Vec<_> = cmds.keys().cloned().collect();
        sections.sort();

        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["Command", "Explanation"]);

        table.add_row(row!["/help", "Display this list"]);
        table.add_row(row!["", ""]);

        for maybe_err in sections.iter() {
            let refs = cmds.get_mut(maybe_err).unwrap();
            refs.sort_by_key(|r| r.usage);

            if let Some(err) = maybe_err {
                let notice = match err {
                    DuringWarmup => "during warmup".to_string(),
                    InMode(script) => format!("in {} mode", script.name()),
                    InOtherModes => "in other modes".to_string(),
                    NoPermission => "no permission".to_string(),
                }
                .to_uppercase();

                table.add_row(row!["", ""]);
                table.add_row(row!["", notice]);
            }

            for r in refs {
                table.add_row(row![r.usage, r.doc]);
            }
        }

        table.to_string()
    }
}
