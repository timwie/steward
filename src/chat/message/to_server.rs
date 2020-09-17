use std::fmt::Display;

use chrono::Duration;
use serde::export::Formatter;

use crate::chat::message::{fmt_time, write_and_reset, write_highlighted, write_start_message};

/// Chat announcements from the controller to all players.
///
/// Note: messages should typically convey information that is
/// not already conveyed by widgets.
pub enum ServerMessage<'a> {
    /// A player connected.
    Joining { nick_name: &'a str },

    /// A player disconnected.
    Leaving { nick_name: &'a str },

    /// At least one player improved their rank, and took one of the top spots.
    NewTopRanks(Vec<TopRankMessage<'a>>),

    /// A player improved their record on the current map,
    /// and took one of the top spots.
    TopRecord {
        player_nick_name: &'a str,
        new_map_rank: usize,
        millis: usize,
    },

    /// A player improved their map record, but kept the same record rank.
    TopRecordImproved {
        player_nick_name: &'a str,
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
}

/// A player improved their rank, and took one of the top spots.
pub struct TopRankMessage<'a> {
    pub nick_name: &'a str,
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
            Joining { nick_name } => {
                write_and_reset(f, nick_name)?;
                write!(f, " joined.")
            }

            Leaving { nick_name } => {
                write_and_reset(f, nick_name)?;
                write!(f, " left.")
            }

            NewTopRanks(top_ranks) => {
                for tr in top_ranks {
                    write_and_reset(f, tr.nick_name)?;
                    writeln!(f, " reaches rank ")?;
                    write_highlighted(f, tr.rank)?;
                    writeln!(f, "!")?;
                }
                Ok(())
            }

            TopRecord {
                player_nick_name: nick_name,
                new_map_rank: new_record_rank,
                millis,
            } => {
                write_and_reset(f, nick_name)?;
                writeln!(f, " sets the ")?;
                write_highlighted(f, format!("{}.", new_record_rank))?;
                writeln!(f, " record! ")?;
                write_highlighted(f, fmt_time(*millis))
            }

            TopRecordImproved {
                player_nick_name: nick_name,
                map_rank: record_rank,
                millis,
            } => {
                write_and_reset(f, nick_name)?;
                writeln!(f, " improved the ")?;
                write_highlighted(f, format!("{}.", record_rank))?;
                writeln!(f, " record! ")?;
                write_highlighted(f, fmt_time(*millis))
            }

            NewMap { name, author } => {
                writeln!(f, "A new map was added: ")?;
                write_and_reset(f, name)?;
                writeln!(f, " by ")?;
                write_and_reset(f, author)
            }

            AddedMap { name } => {
                write_and_reset(f, name)?;
                writeln!(f, " was added back into the playlist.")
            }

            RemovedMap { name } => {
                write_and_reset(f, name)?;
                writeln!(f, " was removed from the playlist.")
            }

            NextMap { name, author } => {
                writeln!(f, "Next map will be ")?;
                write_and_reset(f, name)?;
                writeln!(f, " by ")?;
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
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " skipped the current map!")
            }

            MapDeleted {
                admin_name,
                map_name,
            } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " deleted map ")?;
                write_and_reset(f, map_name)?;
                writeln!(f, " and all of its records!")
            }

            PlayerBlacklisted {
                admin_name,
                player_name,
            } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " blacklisted player ")?;
                write_and_reset(f, player_name)?;
                writeln!(f, "!")
            }

            PlayerUnblacklisted {
                admin_name,
                player_name,
            } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " un-blacklisted player ")?;
                write_and_reset(f, player_name)?;
                writeln!(f, "!")
            }

            ForceRestart { admin_name } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " forced a map restart!")
            }

            ForceQueued {
                admin_name,
                map_name,
            } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " queued map ")?;
                write_and_reset(f, map_name)?;
                writeln!(f, "!")
            }

            TimeLimitChanged { admin_name } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " changed the time limit settings!")
            }

            MatchPaused { admin_name } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " paused the match!")
            }

            MatchUnpaused { admin_name } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " unpaused the match!")
            }

            WarmupRoundExtended { admin_name, secs } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " extended the warmup by ")?;
                write_highlighted(f, format!("{} seconds", secs))?;
                writeln!(f, "!")
            }

            WarmupSkipped { admin_name } => {
                writeln!(f, "Admin ")?;
                write_and_reset(f, admin_name)?;
                writeln!(f, " skipped the warmup!")
            }
        }
    }
}
