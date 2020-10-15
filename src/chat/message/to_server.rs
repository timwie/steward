use std::fmt::Display;

use chrono::Duration;
use serde::export::Formatter;

use crate::chat::message::{fmt_time, write_and_reset, write_highlighted, write_start_message};
use crate::server::ModeScript;

/// Chat announcements from the controller to all players.
///
/// Note: messages should typically convey information that is
/// not already conveyed by widgets.
pub enum ServerMessage<'a> {
    /// A player connected.
    Joining { display_name: &'a str },

    /// A player disconnected.
    Leaving { display_name: &'a str },

    /// At least one player improved their rank, and took one of the top spots.
    NewTopRanks(Vec<TopRankMessage<'a>>),

    /// A player improved their record on the current map,
    /// and took one of the top spots.
    TopRecord {
        player_display_name: &'a str,
        new_map_rank: usize,
        millis: usize,
    },

    /// A player improved their map record, but kept the same record rank.
    TopRecordImproved {
        player_display_name: &'a str,
        map_rank: usize,
        millis: usize,
    },

    /// A new map was imported.
    NewMap { name: &'a str, author: &'a str },

    /// A map was re-introduced to the playlist.
    AddedMap { name: &'a str },

    /// A map was removed from the playlist.
    RemovedMap { name: &'a str },

    /// Announce the next map after the vote.
    NextMap { name: &'a str, author: &'a str },

    /// Tell players to vote if they want a restart.
    VoteNow { duration: Duration, threshold: f32 },

    /// Tell players that an admin skipped the current map.
    CurrentMapSkipped { admin_name: &'a str },

    /// Tell players that an admin deleted a map and its records.
    MapDeleted {
        admin_name: &'a str,
        map_name: &'a str,
    },

    /// Tell players that an admin has blacklisted a player.
    PlayerBlacklisted {
        admin_name: &'a str,
        player_name: &'a str,
    },

    /// Tell players that an admin has removed a player from the blacklist.
    PlayerUnblacklisted {
        admin_name: &'a str,
        player_name: &'a str,
    },

    /// Tell players that an admin has forced a restart of the current map.
    ForceRestart { admin_name: &'a str },

    /// Tell players that an admin has pushed a map to the top of the queue.
    ForceQueued {
        admin_name: &'a str,
        map_name: &'a str,
    },

    /// Tell players that an admin has changed the time limit config.
    TimeLimitChanged { admin_name: &'a str },

    /// Tell players that an admin has paused the match.
    MatchPaused { admin_name: &'a str },

    /// Tell players that an admin has unpaused the match.
    MatchUnpaused { admin_name: &'a str },

    /// Tell players that an admin has extended the current warmup round.
    WarmupRoundExtended { admin_name: &'a str, secs: u64 },

    /// Tell players that an admin has skipped the remaining warmup.
    WarmupSkipped { admin_name: &'a str },

    /// Tell players that an admin changed the game mode for the next map.
    ModeChanging {
        admin_name: &'a str,
        mode: ModeScript,
    },

    /// Tell players that an admin loaded match settings from a file.
    LoadedMatchSettings {
        admin_name: &'a str,
        settings_name: &'a str,
    },

    /// Tell players that an admin saved the current match settings to a file.
    SavedMatchSettings {
        admin_name: &'a str,
        settings_name: &'a str,
    },
}

/// A player improved their rank, and took one of the top spots.
pub struct TopRankMessage<'a> {
    pub display_name: &'a str,
    pub rank: usize,
}

impl Display for ServerMessage<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ServerMessage::*;

        match self {
            NewTopRanks(top_ranks) if top_ranks.is_empty() => return Ok(()),
            _ => {}
        }

        write_start_message(f)?;

        match self {
            Joining { display_name } => {
                write_and_reset(f, display_name)?;
                write!(f, " joined.")
            }

            Leaving { display_name } => {
                write_and_reset(f, display_name)?;
                write!(f, " left.")
            }

            NewTopRanks(top_ranks) => {
                for tr in top_ranks {
                    write_and_reset(f, tr.display_name)?;
                    write!(f, " reaches rank ")?;
                    write_highlighted(f, tr.rank)?;
                    write!(f, "!")?;
                }
                Ok(())
            }

            TopRecord {
                player_display_name: display_name,
                new_map_rank: new_record_rank,
                millis,
            } => {
                write_and_reset(f, display_name)?;
                write!(f, " sets the ")?;
                write_highlighted(f, format!("{}.", new_record_rank))?;
                write!(f, " record! ")?;
                write_highlighted(f, fmt_time(*millis))
            }

            TopRecordImproved {
                player_display_name: display_name,
                map_rank: record_rank,
                millis,
            } => {
                write_and_reset(f, display_name)?;
                write!(f, " improved the ")?;
                write_highlighted(f, format!("{}.", record_rank))?;
                write!(f, " record! ")?;
                write_highlighted(f, fmt_time(*millis))
            }

            NewMap { name, author } => {
                write!(f, "A new map was added: ")?;
                write_and_reset(f, name)?;
                write!(f, " by ")?;
                write_and_reset(f, author)
            }

            AddedMap { name } => {
                write_and_reset(f, name)?;
                write!(f, " was added back into the playlist.")
            }

            RemovedMap { name } => {
                write_and_reset(f, name)?;
                write!(f, " was removed from the playlist.")
            }

            NextMap { name, author } => {
                write!(f, "Next map will be ")?;
                write_and_reset(f, name)?;
                write!(f, " by ")?;
                write_and_reset(f, author)
            }

            VoteNow { threshold, .. } if *threshold > 1f32 => {
                write!(f, "This map will not be restarted.")
            }

            VoteNow { duration, .. } => {
                write!(f, "Vote for a restart in the next ")?;
                write_highlighted(f, format!("{} seconds", duration.num_seconds()))?;
                write!(f, "!")
            }

            CurrentMapSkipped { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " skipped the current map!")
            }

            MapDeleted {
                admin_name,
                map_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " deleted map ")?;
                write_and_reset(f, map_name)?;
                write!(f, " and all of its records!")
            }

            PlayerBlacklisted {
                admin_name,
                player_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " blacklisted player ")?;
                write_and_reset(f, player_name)?;
                write!(f, "!")
            }

            PlayerUnblacklisted {
                admin_name,
                player_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " un-blacklisted player ")?;
                write_and_reset(f, player_name)?;
                write!(f, "!")
            }

            ForceRestart { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " forced a map restart!")
            }

            ForceQueued {
                admin_name,
                map_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " queued map ")?;
                write_and_reset(f, map_name)?;
                write!(f, "!")
            }

            TimeLimitChanged { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " changed the time limit settings!")
            }

            MatchPaused { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " paused the match!")
            }

            MatchUnpaused { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " unpaused the match!")
            }

            WarmupRoundExtended { admin_name, secs } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " extended the warmup by ")?;
                write_highlighted(f, format!("{} seconds", secs))?;
                write!(f, "!")
            }

            WarmupSkipped { admin_name } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " skipped the warmup!")
            }

            ModeChanging { admin_name, mode } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " changed the game mode to ")?;
                write_highlighted(f, mode.name())?;
                write!(f, "!")
            }

            LoadedMatchSettings {
                admin_name,
                settings_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " loaded the ")?;
                write_highlighted(f, settings_name)?;
                write!(f, " match settings!")
            }

            SavedMatchSettings {
                admin_name,
                settings_name,
            } => {
                write!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                write!(f, " saved the current match settings in ")?;
                write_highlighted(f, settings_name)?;
                write!(f, "!")
            }
        }
    }
}
