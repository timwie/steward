use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Duration;

use crate::action::Action;
use crate::command::{AdminCommand, DangerousCommand, PlayerCommand, SuperAdminCommand};
use crate::database::{Map, RecordDetailed};
use crate::ingame::PlayerInfo;

/// This data type is introduced to complement `ServerEvent`s,
/// and to make it easier to understand the controller flow.
/// Many variants contain data that is used to update widgets.
#[derive(Debug)]
pub enum ControllerEvent<'a> {
    /// Signals that a new map was loaded by the server, and the race
    /// is about to begin.
    BeginIntro { loaded_map: Map },

    /// Signals that a player has loaded the map, and is entering
    /// the race by starting their first run. This event is triggered at
    /// the moment they can accelerate.
    EndIntro { player_login: &'a str },

    /// Signals a player starting a run. This event is triggered at
    /// the moment they can accelerate.
    BeginRun { player_login: &'a str },

    /// Signals that a player completed a run, and the start of the
    /// run outro for them.
    EndRun { pb_diff: PbDiff },

    /// Signals the start of the vote, which begins when the race is
    /// concluded, and ends at some point before the map is unloaded.
    BeginOutro { vote: VoteInfo },

    /// Signals that the current map will be unloaded.
    EndOutro,

    /// Signals the end of the vote, and that the next map was decided.
    EndVote { queue_preview: Vec<QueueEntry> },

    /// Signals a player joining, leaving, or transitioning between playing
    /// and spectating.
    NewPlayerList(PlayerDiff),

    /// Signals that the server's playlist was changed.
    NewPlaylist(PlaylistDiff),

    /// Signals that the server ranking has been updated.
    NewServerRanking(ServerRankingDiff),

    /// Signals that a chat command has been issued.
    IssuedCommand(Command<'a>),

    /// Signals that a player has issued an `Action`.
    IssuedAction {
        from_login: &'a str,
        action: Action<'a>,
    },
}

/// Contains information of a player that is either joining, leaving, or
/// transitioning between playing and spectating.
#[derive(Debug)]
pub enum PlayerDiff {
    /// Player joined into a player slot.
    AddPlayer(PlayerInfo),

    /// Player joined into a player slot, but is spectating.
    AddSpectator(PlayerInfo),

    /// Player joined into a spectator slot.
    AddPureSpectator(PlayerInfo),

    /// Player stopped spectating.
    MoveToPlayer(PlayerInfo),

    /// A player started spectating, but still has their player slot.
    MoveToSpectator(PlayerInfo),

    /// A player has moved into a spectator slot.
    MoveToPureSpectator(PlayerInfo),

    /// A player with a player slot has disconnected.
    RemovePlayer(PlayerInfo),

    /// A spectator with a player slot has disconnected.
    RemoveSpectator(PlayerInfo),

    /// A spectator without a player slot has disconnected.
    RemovePureSpectator(PlayerInfo),
}

/// Details of a vote during the outro.
#[derive(Debug)]
pub struct VoteInfo {
    pub min_restart_vote_ratio: f32,
    pub duration: Duration,
}

/// An entry in the map queue, which assigns a priority to a
/// map in the playlist.
#[derive(Debug)]
pub struct QueueEntry {
    pub map: Map,
    pub priority: QueuePriority,
}

/// When deciding the next map, each map is assigned a priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuePriority {
    /// Map was just played, and voted to restart.
    VoteRestart,

    /// Map was force-queued by an admin. The inner number is the
    /// amount of maps that were force-queued ahead of this map.
    /// This priority is used f.e. when importing new maps.
    Force(usize),

    /// Map has a calculated priority.
    Score(i32),

    /// Map was just played, and *not* voted to restart.
    /// Every other map has a higher priority.
    NoRestart,
}

impl Ord for QueuePriority {
    /// `VoteRestart < Force(x) < Force(x+1) < Score(y) < Score(y-1) < NoRestart`
    fn cmp(&self, other: &Self) -> Ordering {
        use QueuePriority::*;
        match (self, other) {
            (VoteRestart, VoteRestart) => Ordering::Equal,
            (NoRestart, NoRestart) => Ordering::Equal,
            (Score(a), Score(b)) => b.cmp(a), // higher score queued first
            (Force(a), Force(b)) => a.cmp(b), // lower pos queued first

            (VoteRestart, _) => Ordering::Less,
            (Force(_), Score(_)) => Ordering::Less,
            (_, NoRestart) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl PartialOrd for QueuePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A change of the server playlist. Only maps in the playlist can be queued.
#[derive(Debug)]
pub enum PlaylistDiff {
    /// Add a map to the playlist that has never been in the playlist before.
    AppendNew(Map),

    /// Add a map to the playlist.
    Append(Map),

    /// Remove a map from the playlist.
    Remove { was_index: usize, map: Map },
}

/// Changes in server ranks for all connected players.
#[derive(Debug)]
pub struct ServerRankingDiff {
    /// Maps players' UIDs to their updated server ranks.
    pub diffs: HashMap<i32, ServerRankDiff>,

    /// The maximum server rank; or the number of players that have a server rank.
    pub max_pos: usize,
}

/// The difference in a player's server rank after a race, if any.
#[derive(Debug, Clone)]
pub struct ServerRankDiff {
    /// The player's formatted nick name.
    pub player_nick_name: String,

    /// The server rank.
    pub new_pos: usize,

    /// The amount of spots gained or lost in the server ranking.
    pub gained_pos: i32,

    /// The amount of *wins* gained or lost. This player gains wins for each
    /// record on each map that was beaten by a record of their own.
    pub gained_wins: i32,
}

/// Compares a player's run to their personal best.
#[derive(Debug)]
pub struct PbDiff {
    /// The UID of the player that set the run.
    pub player_uid: i32,

    /// The millisecond difference of this run vs the player's personal best.
    /// If negative, the player has set a new personal best.
    pub millis_diff: Option<i32>,

    /// The player's map rank before this run.
    pub prev_pos: Option<usize>,

    /// The player's map rank after this run, which has improved
    /// if `new_record` is `Some`, and is equal to `prev_pos` otherwise.
    pub new_pos: usize,

    /// If `prev_pos` is `Some`, this is `prev_pos - new_pos`.
    /// Otherwise, this is the total number of records minus `new_pos`.
    pub pos_gained: usize,

    /// `Some` if the player improved their personal best.
    pub new_record: Option<RecordDetailed>,
}

/// A command with its sender, who was confirmed to have the necessary permission.
#[derive(Debug)]
pub enum Command<'a> {
    Player {
        from: &'a str,
        cmd: PlayerCommand,
    },
    Admin {
        from: &'a str,
        cmd: AdminCommand<'a>,
    },
    SuperAdmin {
        from: &'a str,
        cmd: SuperAdminCommand,
    },
    Dangerous {
        from: &'a str,
        cmd: DangerousCommand,
    },
}
