use std::fmt::{Display, Formatter};

use prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR;
use prettytable::{cell, row, Table};
use semver::Version;

use crate::chat::command::output::truncate;
use crate::chat::CommandContext;
use crate::config::TimeAttackConfig;
use crate::database::{Map, Player};
use crate::server::{PlayerInfo, ServerBuildInfo, ServerNetStats};

/// Outputs for successful commands that list some result.
pub enum CommandResultOutput<'a> {
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
    ControllerInfo(Box<ControllerInfo>),
}

pub struct ControllerInfo {
    pub controller_version: Version,
    pub most_recent_controller_version: Version,
    pub mode_config: TimeAttackConfig,
    pub server_info: ServerBuildInfo,
    pub net_stats: ServerNetStats,
    pub admins: Vec<Player>,
}

impl Display for CommandResultOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CommandResultOutput::*;

        match self {
            CommandReference(ctxt) => write!(f, "{}", ctxt.reference()),

            CurrentConfig { repr } => write!(f, "{}", repr),

            InvalidConfig {
                tried_repr,
                error_msg,
            } => {
                writeln!(f, "# The config you entered is invalid:")?;
                for line in error_msg.lines() {
                    writeln!(f, "# {}", line)?;
                }
                writeln!(f)?;
                write!(f, "{}", tried_repr)
            }

            MapList {
                in_playlist,
                not_in_playlist,
            } => {
                let mut table = Table::new();
                table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
                table.set_titles(row!["Name", "UID", "Link"]);

                let add_row = move |table: &mut Table, map: &Map| {
                    table.add_row(row![
                        truncate(&map.name.plain(), 20),
                        &map.uid,
                        map.exchange_id
                            .map(|id| format!("trackmania.exchange/maps/{}", id))
                            .unwrap_or_default(),
                    ]);
                };

                for map in in_playlist.iter() {
                    add_row(&mut table, map);
                }

                if not_in_playlist.is_empty() {
                    return Ok(());
                }

                table.add_row(row!["", "", ""]);
                table.add_row(row!["not in playlist".to_uppercase(), "", ""]);

                for map in not_in_playlist.iter() {
                    add_row(&mut table, map);
                }

                write!(f, "{}", table.to_string())
            }

            PlayerList(players) => {
                let mut table = Table::new();
                table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
                table.set_titles(row!["Nickname", "Login"]);

                for player in players {
                    table.add_row(row![
                        truncate(&player.display_name.plain(), 30),
                        &player.login
                    ]);
                }

                write!(f, "{}", table.to_string())
            }

            ControllerInfo(info) => {
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
        }
    }
}
