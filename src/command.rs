use std::fmt::{Display, Formatter};
use std::str::FromStr;

use semver::Version;

use gbx::{NetStats, ServerInfo};

use crate::config::Config;
use crate::database::Map;

/// Chat commands that can only be executed by super admins.
#[derive(Debug)]
pub enum SuperAdminCommand {
    /// Print a reference of available super admin commands.
    ///
    /// Usage: `/help`
    Help,

    /// Confirm and execute the previous, dangerous command.
    ///
    /// Usage: `/confirm`
    Confirm,

    /// Prepare a dangerous command, that will only be executed
    /// after using the `/confirm` command.
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
    /// Usage: `/delete_map <uid>`
    DeleteMap { uid: String },

    /// Delete a blacklisted player from the database.
    ///
    /// Usage: `/delete_player <login>`
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
            ["/confirm"] => Some(Confirm),
            ["/delete_map", uid] => Some(Unconfirmed(DeleteMap {
                uid: (*uid).to_string(),
            })),
            ["/delete_player", login] => Some(Unconfirmed(DeletePlayer {
                login: (*login).to_string(),
            })),
            ["/help"] => Some(Help),
            ["/shutdown"] => Some(Unconfirmed(Shutdown)),
            _ => None,
        }
    }
}

/// Chat commands that can only be executed by admins.
#[derive(Debug)]
pub enum AdminCommand<'a> {
    /// Print a reference of available admin commands.
    ///
    /// Usage: `/help`
    Help,

    /// List the server's maps (including UID).
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

    /// End the current race immediately.
    ///
    /// Usage `/skip`
    SkipCurrentMap,

    /// Restart the current map after the race.
    ///
    /// Usage `/restart`
    RestartCurrentMap,

    /// Set the map that will be played after the current one.
    /// Running this command multiple times will queue all maps
    /// in order.
    ///
    /// Usage `/queue <uid>`
    ForceQueue { uid: &'a str },

    /// Set the duration of a race in seconds.
    ///
    /// Usage `/set_timelimit <seconds>`
    SetRaceDuration(u32),

    /// Set the outro duration in seconds.
    ///
    /// Usage `/set_outro <seconds>`
    SetOutroDuration(u32),

    /// Add a player to the server's blacklist, and kick them if they are
    /// currently connected.
    ///
    /// Usage: `/blacklist <login>`
    BlacklistAdd { login: &'a str },

    /// Remove a player from the server's blacklist.
    ///
    /// Usage: `/unblacklist <login>`
    BlacklistRemove { login: &'a str },
}

impl AdminCommand<'_> {
    /// Parse an admin command.
    pub fn from(chat_message: &str) -> Option<AdminCommand> {
        use AdminCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/blacklist", login] => Some(BlacklistAdd { login: *login }),
            ["/help"] => Some(Help),
            ["/map_import", id] => Some(ImportMap { id: *id }),
            ["/maps"] => Some(ListMaps),
            ["/playlist_add", uid] => Some(PlaylistAdd { uid: *uid }),
            ["/playlist_remove", uid] => Some(PlaylistRemove { uid: *uid }),
            ["/queue", uid] => Some(ForceQueue { uid: *uid }),
            ["/restart"] => Some(RestartCurrentMap),
            ["/set_race", secs] if secs.chars().all(|c| c.is_digit(10)) => {
                match u32::from_str(*secs) {
                    Ok(secs) if secs > 0 => Some(SetRaceDuration(secs)),
                    _ => None,
                }
            }
            ["/set_outro", secs] if secs.chars().all(|c| c.is_digit(10)) => {
                match u32::from_str(*secs) {
                    Ok(secs) if secs > 0 => Some(SetOutroDuration(secs)),
                    _ => None,
                }
            }
            ["/skip"] => Some(SkipCurrentMap),
            ["/unblacklist", login] => Some(BlacklistRemove { login: *login }),
            _ => None,
        }
    }
}

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

/// Super admin command reference that can be printed in-game.
pub const SUPER_ADMIN_COMMAND_REFERENCE: &str = "
/confirm                  Confirm the execution of one of dangerous the commands below.
/delete_map <uid>         Delete a map that is not in the playlist from the database. Needs confirmation.
/delete_player <login>    Delete a blacklisted player from the database. Needs confirmation.
/shutdown                 Shutdown the server. Needs confirmation.
";

/// Admin command reference that can be printed in-game.
pub const ADMIN_COMMAND_REFERENCE: &str = "
/map_import <id/uid>       Import the trackmania.exchange map with the given id.
/playlist_add <uid>        Add the specified map to the playlist.
/playlist_remove <uid>     Remove the specified map from the playlist.

/skip            Start the next map immediately.
/restart         Restart the current map after this race.
/queue <uid>     Set the map that will be played after the current one.

/set_timelimit <seconds>     Change the time limit.
/set_outro <seconds>         Change the outro duration at the end of a map.

/blacklist <login>       Add a player to the server's blacklist.
/unblacklist <login>     Remove a player from the server's blacklist.
";

/// Player command reference that can be printed in-game.
pub const PLAYER_COMMAND_REFERENCE: &str = "
/help     Display this list.
/info     Display information about server & controller.
";

/// Possible outputs of chat commands.
pub enum CommandOutput<'a> {
    /// Tell a super admin the command reference.
    ///
    /// Output for: `/help`
    SuperAdminCommandReference,

    /// Tell an admin the command reference.
    ///
    /// Output for: `/help`
    AdminCommandReference,

    /// Tell a player the command reference.
    ///
    /// Output for: `/help`
    PlayerCommandReference,

    /// List all maps in the database, and group maps in- and outside
    /// of the playlist.
    ///
    /// Output for `/maps`
    MapList(Vec<Map>),

    /// Feedback for commands that affect the playlist.
    ///
    /// Output for `/playlist_add`, `/playlist_remove`, `/map_import`
    InvalidPlaylistCommand(PlaylistCommandError),

    /// Information about server & controller.
    ///
    /// Output for `/info`
    Info {
        controller_version: &'a Version,
        most_recent_controller_version: &'a Version,
        config: &'a Config,
        server_info: &'a ServerInfo,
        net_stats: &'a NetStats,
        blacklist: &'a Vec<String>,
    },

    /// The specified login does not match any player.
    ///
    /// Output for `/delete_player`
    UnknownPlayer,

    /// The specified login does not match any blacklisted player.
    ///
    /// Output for `/blacklist`
    UnknownBlacklistPlayer,

    /// Tell a super admin that prior to deleting a player,
    /// they have to blacklist them.
    ///
    /// Output for `/delete_player`
    CannotDeleteWhitelistedPlayer,

    /// The specified map UID does not match any map.
    ///
    /// Output for `/delete_map`, `/queue`
    UnknownMap,

    /// Tell a super admin that prior to deleting a map,
    /// they have to remove it from the playlist.
    ///
    /// Output for `/delete_map`
    CannotDeletePlaylistMap,

    /// Tell a super admin that there is no command to confirm.
    ///
    /// Output for `/confirm`
    NoCommandToConfirm,

    /// Tell a super admin that all records for that map will be deleted.
    ///
    /// Output for `/delete_map`
    ConfirmMapDeletion,

    /// Tell a super admin that all records for that player will be deleted.
    ///
    /// Output for `/delete_player`
    ConfirmPlayerDeletion,

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

impl Display for CommandOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CommandOutput::*;
        use PlaylistCommandError::*;

        match self {
            SuperAdminCommandReference => {
                writeln!(f, "Super admin commands:")?;
                writeln!(f, "===============")?;
                write!(f, "{}", SUPER_ADMIN_COMMAND_REFERENCE)?;
                writeln!(f)?;
                writeln!(f, "Admin commands:")?;
                writeln!(f, "===============")?;
                write!(f, "{}", ADMIN_COMMAND_REFERENCE)?;
                writeln!(f)?;
                writeln!(f, "Player commands:")?;
                writeln!(f, "================")?;
                write!(f, "{}", PLAYER_COMMAND_REFERENCE)
            }

            AdminCommandReference => {
                writeln!(f, "Admin commands:")?;
                writeln!(f, "===============")?;
                write!(f, "{}", ADMIN_COMMAND_REFERENCE)?;
                writeln!(f)?;
                writeln!(f, "Player commands:")?;
                writeln!(f, "================")?;
                write!(f, "{}", PLAYER_COMMAND_REFERENCE)
            }

            PlayerCommandReference => write!(f, "{}", PLAYER_COMMAND_REFERENCE),

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
                    writeln!(
                        f,
                        "{} | {} | {}",
                        fill(&map.file_name, 30),
                        &map.uid,
                        map.exchange_id.map(|id| id.to_string()).unwrap_or_default()
                    )?;
                }
                writeln!(f)?;
                writeln!(f, "Not in playlist:")?;
                writeln!(f, "================")?;
                for map in maps.iter().filter(|map| !map.in_playlist) {
                    writeln!(
                        f,
                        "{} | {} | {}",
                        fill(&map.file_name, 30),
                        &map.uid,
                        map.exchange_id.map(|id| id.to_string()).unwrap_or_default()
                    )?;
                }
                Ok(())
            }
            Info {
                controller_version,
                most_recent_controller_version,
                config,
                server_info,
                net_stats,
                blacklist,
            } => {
                writeln!(
                    f,
                    "Controller: $L[https://github.com/timwie/steward]Steward"
                )?;
                writeln!(f, "Controller version: {}", controller_version)?;
                writeln!(
                    f,
                    "Most recent controller version: {}",
                    most_recent_controller_version
                )?;
                writeln!(f)?;

                writeln!(f, "Uptime: {} hours", net_stats.uptime_secs / 60 / 60)?;
                writeln!(f, "{:?}", server_info)?;
                writeln!(f)?;

                writeln!(f, "Time limit: {} seconds", config.race_duration_secs)?;
                writeln!(f, "Outro duration: {} seconds", config.race_duration_secs)?;
                writeln!(
                    f,
                    "Outro vote duration: {} seconds",
                    config.vote_duration_secs()
                )?;
                writeln!(f)?;

                writeln!(
                    f,
                    "Super Admins: {}",
                    config.super_admin_whitelist.join(", ")
                )?;
                writeln!(f, "Admins: {}", config.admin_whitelist.join(", "))?;
                writeln!(f, "Blacklisted: {}", blacklist.join(", "))
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

            NoCommandToConfirm => {
                writeln!(f, "No command to confirm. Use /confirm only when prompted.")
            }

            ConfirmMapDeletion => writeln!(
                f,
                "Use /confirm to delete this map, and all of its records."
            ),

            ConfirmPlayerDeletion => writeln!(
                f,
                "Use /confirm to delete this map, and all of their records."
            ),

            ConfirmShutdown => writeln!(f, "Use /confirm to stop the server."),
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
