use std::fmt::Display;
use std::time::Duration;

use serde::export::Formatter;

use crate::chat::message::{fmt_time, HIGHLIGHT, NOTICE, RESET};
use crate::config::{
    MAX_ANNOUNCED_RANK, MAX_ANNOUNCED_RECORD, MAX_ANNOUNCED_RECORD_IMPROVEMENT,
    MAX_NB_ANNOUNCED_RANKS,
};
use crate::event::{ControllerEvent, PbDiff, PlayerDiff, PlaylistDiff, ServerRankingDiff};

/// Chat announcements from the controller to all players.
/// Many variants can be derived from `ControllerEvent`s.
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

    /// Display some info on the current map ahead of a race.
    CurrentMap { name: &'a str, author: &'a str },

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
}

pub struct TopRankMessage<'a> {
    nick_name: &'a str,
    rank: usize,
}

impl ServerMessage<'_> {
    pub fn from_event<'a>(event: &'a ControllerEvent) -> Option<ServerMessage<'a>> {
        use ControllerEvent::*;
        use ServerMessage::*;

        match event {
            NewPlayerList(PlayerDiff::AddPlayer(info))
            | NewPlayerList(PlayerDiff::AddSpectator(info))
            | NewPlayerList(PlayerDiff::AddPureSpectator(info)) => Some(Joining {
                nick_name: &info.nick_name.formatted,
            }),

            NewPlayerList(PlayerDiff::RemovePlayer(info))
            | NewPlayerList(PlayerDiff::RemoveSpectator(info))
            | NewPlayerList(PlayerDiff::RemovePureSpectator(info)) => Some(Leaving {
                nick_name: &info.nick_name.formatted,
            }),

            BeginMap { loaded_map } => Some(CurrentMap {
                name: &loaded_map.name.formatted,
                author: &loaded_map.author_login,
            }),

            BeginOutro { vote } => Some(VoteNow {
                duration: vote.duration,
                threshold: vote.min_restart_vote_ratio,
            }),

            EndRun {
                pb_diff:
                    PbDiff {
                        new_pos,
                        pos_gained,
                        new_record: Some(new_record),
                        ..
                    },
            } if *pos_gained > 0 && *new_pos <= MAX_ANNOUNCED_RECORD => Some(TopRecord {
                player_nick_name: &new_record.player_nick_name.formatted,
                new_map_rank: *new_pos,
                millis: new_record.millis as usize,
            }),

            EndRun {
                pb_diff:
                    PbDiff {
                        new_pos,
                        pos_gained,
                        new_record: Some(new_record),
                        millis_diff: Some(diff),
                        ..
                    },
            } if *pos_gained == 0 && *diff < 0 && *new_pos <= MAX_ANNOUNCED_RECORD_IMPROVEMENT => {
                Some(TopRecordImproved {
                    player_nick_name: &new_record.player_nick_name.formatted,
                    map_rank: *new_pos,
                    millis: new_record.millis as usize,
                })
            }

            NewPlaylist(PlaylistDiff::AppendNew(map)) => Some(NewMap {
                name: &map.name.formatted,
                author: &map.author_login,
            }),

            NewPlaylist(PlaylistDiff::Append(map)) => Some(AddedMap {
                name: &map.name.formatted,
            }),

            NewPlaylist(PlaylistDiff::Remove { map, .. }) => Some(RemovedMap {
                name: &map.name.formatted,
            }),

            NewServerRanking(ServerRankingDiff { diffs, .. }) => {
                let mut top_ranks: Vec<TopRankMessage> = diffs
                    .values()
                    .filter_map(|diff| {
                        if diff.gained_pos > 0 && diff.new_pos <= MAX_ANNOUNCED_RANK {
                            Some(TopRankMessage {
                                nick_name: &diff.player_nick_name.formatted,
                                rank: diff.new_pos,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                top_ranks.sort_by_key(|tr| tr.rank); // lowest ranks (highest number) last
                top_ranks = top_ranks.into_iter().take(MAX_NB_ANNOUNCED_RANKS).collect();
                top_ranks.reverse(); // highest ranks last -> more prominent in chat
                Some(NewTopRanks(top_ranks))
            }

            _ => None,
        }
    }
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
                "A new map was added: {}{}{} by {}",
                name, RESET, NOTICE, author
            ),

            AddedMap { name: display_name } => write!(
                f,
                "{}{}{} was added back into the playlist.",
                display_name, RESET, NOTICE
            ),

            RemovedMap { name: display_name } => write!(
                f,
                "{}{}{} was removed from the playlist.",
                display_name, RESET, NOTICE
            ),

            CurrentMap {
                name: display_name,
                author,
            } => write!(
                f,
                "Current map is {}{}{} by {}",
                display_name, RESET, NOTICE, author
            ),

            VoteNow { threshold, .. } if *threshold > 1f32 => {
                write!(f, "This map will not be restarted.")
            }

            VoteNow { duration, .. } => write!(
                f,
                "Vote for a restart in the next {} seconds.",
                duration.as_secs()
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
        }
    }
}
