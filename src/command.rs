use std::fmt::{Display, Formatter};

use crate::database::Map;

/// Chat commands that can only be executed by admins.
#[derive(Debug)]
pub enum AdminCommand<'a> {
    /// Print a reference of available commands to the chat.
    ///
    /// Usage: `/help`
    Help,

    /// List the server's maps (including UID) in the chat.
    /// For each map, it should say whether it is in the playlist
    /// or not.
    ///
    /// Usage: `/maps`
    ListMaps,

    /// Add the map with the given UID to the playlist.
    ///
    /// Usage: `/playlist_add <uid>`
    PlaylistAdd { uid: &'a str },

    /// Remove the map with the given UID from the playlist.
    ///
    /// Usage: `/playlist_remove <uid>`
    PlaylistRemove { uid: &'a str },

    /// Import a map from `trackmania.exchange`.
    /// The ID is either its ID on the website (a number),
    /// or its UID (from inside the map file).
    ///
    /// Should have the following outcomes:
    /// - Download the map.
    /// - Add it to the database & playlist.
    /// - Queue it as the next map.
    ///
    /// Usage: `/map_import <id/uid>`
    ImportMap { id: &'a str },
}

impl AdminCommand<'_> {
    /// Parse an admin command. This returns `None` only for chat messages
    /// that do not start with a `/`. For any messages that do start with `/`,
    /// but are not known commands, `Some(Help)` will be returned.
    pub fn from(chat_message: &str) -> Option<AdminCommand> {
        use AdminCommand::*;

        if !chat_message.starts_with('/') {
            return None;
        }
        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/maps"] => Some(ListMaps),
            ["/map_import", id] => Some(ImportMap { id: *id }),
            ["/playlist_add", id] => Some(PlaylistAdd { uid: *id }),
            ["/playlist_remove", id] => Some(PlaylistRemove { uid: *id }),
            _ => Some(Help),
        }
    }
}

/// Chat commands for players.
#[derive(Debug)]
pub enum PlayerCommand {
    // player commands would go here
}

impl PlayerCommand {
    /// Always returns `None`, since there are no player commands yet.
    pub fn from(_chat_message: &str) -> Option<PlayerCommand> {
        None // update in case we add player commands
    }
}

/// Possible outputs of chat commands.
pub enum CommandOutput {
    /// Tell a player the command reference, f.e. when
    /// they issued an unknown command.
    CommandReference,

    /// Response to the `/maps` command.
    MapList(Vec<Map>),

    /// Feedback for commands that affect the playlist:
    /// `/playlist_add`, `/playlist_remove`, `/map_import`
    InvalidPlaylistCommand(PlaylistCommandError),
}

/// Possible errors when issuing a command that changes the playlist.
pub enum PlaylistCommandError {
    /// A generic error message that indicates that the map
    /// UID that was passed to a command was not known.
    UnknownUid,

    /// The map ID or UID used for the map import command
    /// was not known.
    UnknownImportId,

    /// An error message sent to admins that try to disable
    /// the last enabled map. An empty playlist
    /// would cause all kinds of problems.
    CannotDisableAllMaps,

    /// Tried to import a map that was already imported.
    MapAlreadyImported,

    /// Tried to add a map that was already in the playlist.
    MapAlreadyAdded,

    /// Tried to remove a map that was already removed from the playlist.
    MapAlreadyRemoved,

    /// Command failed for a reason not covered by any other variant.
    MapImportFailed(Box<dyn std::error::Error + Send>),
}

impl Display for CommandOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CommandOutput::*;
        use PlaylistCommandError::*;

        match self {
            CommandReference => write!(f, "{}", COMMAND_REFERENCE),

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

            InvalidPlaylistCommand(CannotDisableAllMaps) => write!(
                f,
                "You cannot disable every map! Enable at least one other map."
            ),

            MapList(maps) => {
                writeln!(f, "In playlist:")?;
                writeln!(f, "============")?;
                for map in maps.iter().filter(|map| map.in_playlist) {
                    writeln!(f, "{} | {}", fill(&map.file_name, 30), map.uid)?;
                }
                writeln!(f)?;
                writeln!(f, "Not in playlist:")?;
                writeln!(f, "================")?;
                for map in maps.iter().filter(|map| !map.in_playlist) {
                    writeln!(f, "{} | {}", fill(&map.file_name, 30), map.uid)?;
                }
                Ok(())
            }
        }
    }
}

/// Command reference that can be printed in-game.
pub const COMMAND_REFERENCE: &str = "
Admin Commands:
 -/help                  Display this list.
 -/playlist_remove <uid> Exclude the map with the given uid from playing.
 -/playlist_add <uid>    Allow the map with the given uid to be played.
 -/map_import <id/uid>   Import and enable the trackmania.exchange map with the given id.
";

/// Trims a text, or adds spaces to right, to fill the specified number of columns.
fn fill(text: &str, columns: usize) -> String {
    if text.len() > columns {
        text.chars().take(columns - 1).chain("…".chars()).collect()
    } else {
        text.chars()
            .chain(std::iter::repeat(' ').take(columns - text.len()))
            .collect()
    }
}
