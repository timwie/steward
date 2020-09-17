use std::fmt::Display;

use chrono::Duration;
use serde::export::Formatter;

use crate::chat::message::{fmt_time, HIGHLIGHT, NOTICE, RESET};

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

        write!(f, "{}{}🔊 ", RESET, NOTICE)?;
        match self {
            Joining { nick_name } => write!(f, "{}{}{} joined.", nick_name, RESET, NOTICE),

            Leaving { nick_name } => write!(f, "{}{}{} left.", nick_name, RESET, NOTICE),

            NewTopRanks(top_ranks) => {
                for tr in top_ranks {
                    writeln!(
                        f,
                        "{}{}{} reaches rank {}!",
                        tr.nick_name, RESET, NOTICE, tr.rank
                    )?;
                }
                Ok(())
            }

            TopRecord {
                player_nick_name: nick_name,
                new_map_rank: new_record_rank,
                millis,
            } => write!(
                f,
                "{}{}{} sets the {}. record! {}{}",
                nick_name,
                RESET,
                NOTICE,
                new_record_rank,
                HIGHLIGHT,
                fmt_time(*millis)
            ),

            TopRecordImproved {
                player_nick_name: nick_name,
                map_rank: record_rank,
                millis,
            } => write!(
                f,
                "{}{}{} improved the {}. record! {}{}",
                nick_name,
                RESET,
                NOTICE,
                record_rank,
                HIGHLIGHT,
                fmt_time(*millis)
            ),

            NewMap { name, author } => write!(
                f,
                "A new map was added: {}{}{}{} by {}{}{}",
                RESET, name, RESET, NOTICE, RESET, author, RESET
            ),

            AddedMap { name } => write!(
                f,
                "{}{}{} was added back into the playlist.",
                name, RESET, NOTICE
            ),

            RemovedMap { name } => write!(
                f,
                "{}{}{} was removed from the playlist.",
                name, RESET, NOTICE
            ),

            NextMap { name, author } => write!(
                f,
                "Next map will be {}{}{}{} by {}{}{}",
                RESET, name, RESET, NOTICE, RESET, author, RESET
            ),

            VoteNow { threshold, .. } if *threshold > 1f32 => {
                write!(f, "This map will not be restarted.")
            }

            VoteNow { duration, .. } => write!(
                f,
                "Vote for a restart in the next {} seconds.",
                duration.num_seconds()
            ),

            CurrentMapSkipped { admin_name } => write!(
                f,
                "Admin {}{}{} skipped the current map!",
                admin_name, RESET, NOTICE
            ),

            MapDeleted {
                admin_name,
                map_name,
            } => write!(
                f,
                "Admin {}{}{} deleted {}{}{} and all of its records!",
                admin_name, RESET, NOTICE, map_name, RESET, NOTICE
            ),

            PlayerBlacklisted {
                admin_name,
                player_name,
            } => write!(
                f,
                "Admin {}{}{} blacklisted player {}{}{}!",
                admin_name, RESET, NOTICE, player_name, RESET, NOTICE
            ),

            PlayerUnblacklisted {
                admin_name,
                player_name,
            } => write!(
                f,
                "Admin {}{}{} un-blacklisted player {}{}{}!",
                admin_name, RESET, NOTICE, player_name, RESET, NOTICE
            ),

            ForceRestart { admin_name } => write!(
                f,
                "Admin {}{}{} forced a map restart!",
                admin_name, RESET, NOTICE
            ),

            ForceQueued {
                admin_name,
                map_name,
            } => write!(
                f,
                "Admin {}{}{} queued map {}{}{}!",
                admin_name, RESET, NOTICE, map_name, RESET, NOTICE
            ),

            TimeLimitChanged { admin_name } => write!(
                f,
                "Admin {}{}{} changed the time limit settings!",
                admin_name, RESET, NOTICE
            ),

            MatchPaused { admin_name } => write!(
                f,
                "Admin {}{}{} paused the match!",
                admin_name, RESET, NOTICE
            ),

            MatchUnpaused { admin_name } => write!(
                f,
                "Admin {}{}{} unpaused the match!",
                admin_name, RESET, NOTICE
            ),

            WarmupRoundExtended { admin_name, secs } => write!(
                f,
                "Admin {}{}{} extended the warmup by {} seconds!",
                admin_name, RESET, NOTICE, secs
            ),

            WarmupSkipped { admin_name } => write!(
                f,
                "Admin {}{}{} skipped the warmup!",
                admin_name, RESET, NOTICE
            ),
        }
    }
}
