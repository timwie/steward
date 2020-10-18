use std::fmt::{Display, Formatter};

use crate::chat::{BadCommandContext, CommandContext, CommandDeniedError};
use crate::server::ModeScript;

/// Outputs for failed commands.
pub enum CommandErrorOutput<'a> {
    /// The reason why a command was not executed.
    CommandError(CommandContext<'a>, CommandDeniedError),

    /// Feedback for commands that affect the playlist.
    ///
    /// Output for `/playlist add`, `/playlist remove`, `/import map`
    InvalidPlaylistCommand(PlaylistCommandError),

    /// The specified login does not match any player.
    ///
    /// Output for `/delete player`, `/kick`, `/bounce`
    UnknownPlayer,

    /// The specified login does not match any blacklisted player.
    ///
    /// Output for `/blacklist remove`
    UnknownBlacklistPlayer,

    /// Tell a super admin that prior to deleting a player,
    /// they have to blacklist them.
    ///
    /// Output for `/delete player`
    CannotDeleteWhitelistedPlayer,

    /// The specified map UID does not match any map.
    ///
    /// Output for `/delete map`, `/queue`
    UnknownMap,

    /// Tell a super admin that prior to deleting a map,
    /// they have to remove it from the playlist.
    ///
    /// Output for `/delete map`
    CannotDeletePlaylistMap,

    /// Tell an admin that the current game mode does not support pauses.
    ///
    /// Output for `/pause`
    CannotPause,

    /// Tell an admin that the entered command only works during warmup.
    ///
    /// Output for any `/warmup *` command
    NotInWarmup,

    /// Tell an admin that the given name did not match any available game mode.
    ///
    /// Output for `/mode` command
    UnknownMode {
        tried: &'a str,
        options: Vec<ModeScript>,
    },

    /// Tell an admin that the mode could not be changed.
    ///
    /// Output for `/mode` command
    CannotChangeMode { msg: &'a str },

    /// Tell an admin that the given name did not match any available match settings.
    ///
    /// Output for `/settings load` command
    UnknownMatchSettings {
        tried: &'a str,
        options: Vec<String>,
    },

    /// Tell an admin that the current match settings could not be saved to a file.
    ///
    /// Output for `/settings save` command
    CannotSaveMatchSettings { msg: &'a str },
}

/// Possible errors when issuing a command that changes the playlist.
pub enum PlaylistCommandError {
    /// A generic error message that indicates that the map
    /// UID that was passed to a command was not known.
    UnknownUid,

    /// The map ID or UID used for the map import command
    /// was not known.
    UnknownImportId,

    /// An error message sent to admins that try to remove the single map
    /// currently in the playlist.
    EmptyPlaylistDisallowed,

    /// Tried to import a map that was already imported.
    MapAlreadyImported,

    /// Tried to add a map that was already in the playlist.
    MapAlreadyAdded,

    /// Tried to remove a map that was already removed from the playlist.
    MapAlreadyRemoved,

    /// Command failed for a reason not covered by any other variant.
    MapImportFailed(Box<dyn std::error::Error + Send>),
}

impl Display for CommandErrorOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use crate::chat::CommandDeniedError::*;
        use CommandErrorOutput::*;
        use PlaylistCommandError::*;

        match self {
            InvalidPlaylistCommand(UnknownUid) => write!(f, "No server map with this UID!"),

            InvalidPlaylistCommand(UnknownImportId) => {
                write!(f, "No map with this ID or UID on Trackmania.Exchange!")
            }

            InvalidPlaylistCommand(MapAlreadyImported) => {
                write!(f, "This map was already imported.")
            }

            InvalidPlaylistCommand(MapAlreadyAdded) => {
                write!(f, "This map is already in the playlist.")
            }

            InvalidPlaylistCommand(MapAlreadyRemoved) => {
                write!(f, "This map was already removed from the playlist.")
            }

            InvalidPlaylistCommand(MapImportFailed(err)) => {
                write!(f, "Failed to import map: {:?}", err)
            }

            InvalidPlaylistCommand(EmptyPlaylistDisallowed) => {
                write!(f, "You cannot remove the only map in the playlist. ")?;
                write!(f, "Add at least one other map to remove this one.")
            }

            CommandError(ctxt, NotAvailable(err)) => {
                match err {
                    BadCommandContext::DuringWarmup => {
                        writeln!(f, "The '{}' command only works during warmups", ctxt.cmd)
                    }
                    BadCommandContext::InMode(script) => writeln!(
                        f,
                        "The '{}' command is for the '{}' mode only",
                        ctxt.cmd,
                        script.name()
                    ),
                    BadCommandContext::InOtherModes => {
                        writeln!(f, "You cannot use '{}' in this game mode", ctxt.cmd)
                    }
                    BadCommandContext::NoPermission => {
                        writeln!(f, "You are not permitted to use the '{}' command", ctxt.cmd)
                    }
                }?;
                writeln!(f)?;
                write!(f, "{}", ctxt.reference())
            }
            CommandError(ctxt, NoSuchCommand) => {
                writeln!(f, "'{}' is not a valid command", ctxt.cmd)?;
                writeln!(f)?;
                write!(f, "{}", ctxt.reference())
            }

            UnknownPlayer => writeln!(f, "There is no player with that login!"),

            UnknownBlacklistPlayer => {
                writeln!(f, "There is no blacklisted player with that login!")
            }

            CannotDeleteWhitelistedPlayer => writeln!(
                f,
                "Only blacklisted players can be removed from the database!"
            ),

            UnknownMap => writeln!(f, "There is no map with that UID!"),

            CannotDeletePlaylistMap => writeln!(
                f,
                "Only maps outside of the playlist can be removed from the database!"
            ),

            CannotPause => writeln!(f, "This game mode does not support pausing!"),

            NotInWarmup => writeln!(f, "This command works only during warmup."),

            UnknownMode { tried, options } => {
                writeln!(f, "'{}' is not a known game mode.", tried)?;
                writeln!(f)?;
                writeln!(f, "The available game modes are:")?;

                for option in options {
                    writeln!(f, " - {}", option.name())?;
                }

                Ok(())
            }

            CannotChangeMode { msg } => writeln!(f, "Failed to change game mode: {}", msg),

            UnknownMatchSettings { tried, options } => {
                writeln!(f, "'{}' is not a known match settings file.", tried)?;
                writeln!(f)?;
                writeln!(f, "The available match settings are:")?;

                for option in options {
                    writeln!(f, " - {}", option)?;
                }

                Ok(())
            }

            CannotSaveMatchSettings { msg } => {
                writeln!(f, "Failed to save match settings: {}", msg)
            }
        }
    }
}
