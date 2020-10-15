use std::fmt::{Display, Formatter};

use semver::Version;

use crate::chat::{BadCommandContext, CommandContext, CommandDeniedError, DangerousCommand};
use crate::config::TimeAttackConfig;
use crate::database::{Map, Player};
use crate::server::{PlayerInfo, ServerBuildInfo, ServerNetStats};
use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;
use prettytable::{cell, row, Table};

/// Possible responses for chat commands.
pub enum CommandResponse<'a> {
    /// Responses for successful commands that give some output.
    Output(CommandOutputResponse<'a>),

    /// Responses for dangerous commands that need confirmation.
    Confirm(DangerousCommand<'a>, CommandConfirmResponse<'a>),

    /// Responses for failed commands.
    Error(CommandErrorResponse<'a>),
}

pub enum CommandOutputResponse<'a> {
    /// Tell a player the command reference.
    ///
    /// Output for: `/help`
    CommandReference(CommandContext<'a>),

    /// List the current config, so that an admin can edit it.
    ///
    /// Output for: `/config`
    CurrentConfig { repr: &'a str },

    /// Tell an admin that the config they submitted was invalid.
    ///
    /// Output for: `/config`, after submitting an invalid config
    InvalidConfig {
        tried_repr: &'a str,
        error_msg: &'a str,
    },

    /// List all maps in the database, and group maps in- and outside
    /// of the playlist.
    ///
    /// Output for `/maps`
    MapList {
        in_playlist: Vec<&'a Map>,
        not_in_playlist: Vec<&'a Map>,
    },

    /// Lists logins and display names of connected players.
    ///
    /// Output for `/players`
    PlayerList(Vec<&'a PlayerInfo>),

    /// Information about server & controller.
    ///
    /// Output for `/info`
    Info(Box<InfoResponse>),
}

pub struct InfoResponse {
    pub controller_version: Version,
    pub most_recent_controller_version: Version,
    pub mode_config: TimeAttackConfig,
    pub server_info: ServerBuildInfo,
    pub net_stats: ServerNetStats,
    pub admins: Vec<Player>,
}

pub enum CommandErrorResponse<'a> {
    /// The reason why a command was not executed.
    CommandError(CommandContext<'a>, CommandDeniedError),

    /// Feedback for commands that affect the playlist.
    ///
    /// Output for `/playlist add`, `/playlist remove`, `/map_import`
    InvalidPlaylistCommand(PlaylistCommandError),

    /// The specified login does not match any player.
    ///
    /// Output for `/delete player`
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
}

pub enum CommandConfirmResponse<'a> {
    /// Tell a super admin that all records for that map will be deleted.
    ///
    /// Output for `/delete map`
    ConfirmMapDeletion { file_name: &'a str },

    /// Tell a super admin that all records for that player will be deleted.
    ///
    /// Output for `/delete player`
    ConfirmPlayerDeletion { login: &'a str },

    /// Tell a super admin that the server will shutdown.
    ///
    /// Output for `/shutdown`
    ConfirmShutdown,
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

impl Display for CommandResponse<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use crate::chat::CommandDeniedError::*;
        use CommandConfirmResponse::*;
        use CommandErrorResponse::*;
        use CommandOutputResponse::*;
        use CommandResponse::*;
        use PlaylistCommandError::*;

        match self {
            Output(CommandReference(ctxt)) => write!(f, "{}", ctxt.reference()),

            Output(CurrentConfig { repr }) => write!(f, "{}", repr),

            Output(InvalidConfig {
                tried_repr,
                error_msg,
            }) => {
                writeln!(f, "# The config you entered is invalid:")?;
                for line in error_msg.lines() {
                    writeln!(f, "# {}", line)?;
                }
                writeln!(f)?;
                write!(f, "{}", tried_repr)
            }

            Error(InvalidPlaylistCommand(UnknownUid)) => write!(f, "No server map with this UID!"),

            Error(InvalidPlaylistCommand(UnknownImportId)) => {
                write!(f, "No map with this ID or UID on Trackmania.Exchange!")
            }

            Error(InvalidPlaylistCommand(MapAlreadyImported)) => {
                write!(f, "This map was already imported.")
            }

            Error(InvalidPlaylistCommand(MapAlreadyAdded)) => {
                write!(f, "This map is already in the playlist.")
            }

            Error(InvalidPlaylistCommand(MapAlreadyRemoved)) => {
                write!(f, "This map was already removed from the playlist.")
            }

            Error(InvalidPlaylistCommand(MapImportFailed(err))) => {
                write!(f, "Failed to import map: {:?}", err)
            }

            Error(InvalidPlaylistCommand(CannotDisableAllMaps)) => write!(
                f,
                "You cannot disable every map! Enable at least one other map."
            ),

            Output(MapList {
                in_playlist,
                not_in_playlist,
            }) => {
                let mut table = Table::new();
                table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
                table.set_titles(row!["Name", "UID", "Link"]);

                let add_row = move |table: &mut Table, map: &Map| {
                    table.add_row(row![
                        fill(&map.name.plain(), 20),
                        &map.uid,
                        map.exchange_id
                            .map(|id| format!("trackmania.exchange/maps/{}", id))
                            .unwrap_or_default(),
                    ]);
                };

                for map in in_playlist.iter() {
                    add_row(&mut table, map);
                }

                table.add_row(row!["", "", ""]);
                table.add_row(row!["not in playlist".to_uppercase(), "", ""]);

                for map in not_in_playlist.iter() {
                    add_row(&mut table, map);
                }

                write!(f, "{}", table.to_string())
            }

            Output(PlayerList(players)) => {
                let mut table = Table::new();
                table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
                table.set_titles(row!["Nickname", "Login"]);

                for player in players {
                    table.add_row(row![fill(&player.display_name.plain(), 30), &player.login]);
                }

                write!(f, "{}", table.to_string())
            }

            Output(Info(info)) => {
                writeln!(
                    f,
                    "This server uses the 'Steward' controller (https://github.com/timwie/steward)"
                )?;
                writeln!(f)?;
                writeln!(f, "Controller version: {}", info.controller_version)?;
                writeln!(
                    f,
                    "Most recent version: {}",
                    info.most_recent_controller_version
                )?;
                writeln!(f)?;

                writeln!(f, "Uptime: {} hours", info.net_stats.uptime_secs / 60 / 60)?;
                writeln!(f, "{:#?}", info.server_info)?;
                writeln!(f)?;

                writeln!(f, "Config:")?;
                write!(f, "{}", info.mode_config.to_string())?;
                writeln!(f)?;

                let names: Vec<String> =
                    info.admins.iter().map(|p| p.display_name.plain()).collect();
                writeln!(f, "Admins: {}", names.join(", "))
            }

            Error(CommandError(ctxt, NotAvailable(err))) => {
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
            Error(CommandError(ctxt, NoSuchCommand)) => {
                writeln!(f, "'{}' is not a valid command", ctxt.cmd)?;
                writeln!(f)?;
                write!(f, "{}", ctxt.reference())
            }

            Error(UnknownPlayer) => writeln!(f, "There is no player with that login!"),

            Error(UnknownBlacklistPlayer) => {
                writeln!(f, "There is no blacklisted player with that login!")
            }

            Error(CannotDeleteWhitelistedPlayer) => writeln!(
                f,
                "Only blacklisted players can be removed from the database!"
            ),

            Error(UnknownMap) => writeln!(f, "There is no map with that UID!"),

            Error(CannotDeletePlaylistMap) => writeln!(
                f,
                "Only maps outside of the playlist can be removed from the database!"
            ),

            Error(CannotPause) => writeln!(f, "This game mode does not support pausing!"),

            Error(NotInWarmup) => writeln!(f, "This command works only during warmup."),

            Confirm(_, ConfirmMapDeletion { file_name }) => writeln!(
                f,
                "Warning: this action will delete map '{}', and all of its records.",
                file_name
            ),

            Confirm(_, ConfirmPlayerDeletion { login }) => writeln!(
                f,
                "Warning: this action will delete player '{}', and all of their records.",
                login
            ),

            Confirm(_, ConfirmShutdown) => writeln!(f, "Warning: this will stop the server."),
        }
    }
}

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
