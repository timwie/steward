use std::collections::HashMap;

use crate::chat::{AdminCommand, PlayerCommand, SuperAdminCommand};
use crate::database::{Map, Record};
use crate::server::CheckpointEvent;
use crate::server::{DisplayString, PlayerInfo};
use crate::widget::Action;

/// This data type is introduced to complement `ServerEvent`s,
/// and to make it easier to understand the controller flow.
#[derive(Debug)]
pub enum ControllerEvent<'a> {
    /// Signals that the current map will be unloaded, and the next
    /// will be loaded.
    ChangeMap,

    /// Signals that a new race is about to begin.
    BeginIntro,

    /// Signals that a player has loaded the map, and is entering
    /// the race by starting their first run. This event is triggered at
    /// the moment they can accelerate.
    EndIntro { player_login: &'a str },

    /// Signals a player starting a run. This event is triggered at
    /// the moment they can accelerate.
    BeginRun { player_login: &'a str },

    /// Signals that a player crossed a checkpoint or finish line.
    ContinueRun(CheckpointEvent),

    /// Signals when a player & server's run times are out of sync. This is likely caused
    /// by connection issues, but could also be a cheating attempt.
    DesyncRun { player_login: &'a str },

    /// Signals that a player completed a run, and the start of the
    /// run outro for them.
    FinishRun(PbDiff),

    /// Signals the start of the vote, which begins when the race is
    /// concluded, and ends at some point before the map is unloaded.
    BeginOutro,

    /// Signals that the outro has ended, and that the current map will
    /// either restart, or will be unloaded.
    EndOutro,

    /// Signals the start of the vote, which coincides with the start
    /// of the outro.
    BeginVote,

    /// Signals the end of the vote, and that the next map was decided.
    EndVote,

    /// Signals that the map queue has changed.
    NewQueue(QueueDiff),

    /// Signals a player joining, leaving, or transitioning between playing
    /// and spectating.
    NewPlayerList(PlayerDiff),

    /// Signals that the server's playlist was changed.
    NewPlaylist(PlaylistDiff),

    /// Signals that the server ranking has been updated.
    NewServerRanking(ServerRankingDiff),

    /// Signals that an admin edited the controller config.
    NewConfig {
        from_login: &'a str,
        change: ConfigDiff,
    },

    /// Signals that a chat command has been issued.
    IssueCommand(Command<'a>),

    /// Signals that a player has issued an `Action`.
    IssueAction { from_login: &'a str, action: Action },

    /// Signals that the warmup section ahead of a match has begun.
    BeginWarmup,

    /// Signals that the warmup section ahead of a match has ended.
    EndWarmup,

    /// Signals that the match was paused.
    BeginPause,

    /// Signals that the match was unpaused.
    EndPause,

    /// Signals that the game mode has changed.
    ChangeMode,
}

/// Contains information of a player that is either joining, leaving, or
/// transitioning between playing and spectating.
#[derive(Debug)]
pub struct PlayerDiff {
    pub info: PlayerInfo,
    pub transition: PlayerTransition,
}

/// Transitions convey whether a player is joining, leaving, or
/// transitioning between playing and spectating.
#[derive(Debug, PartialEq)]
pub enum PlayerTransition {
    /// Player joined into a player slot.
    AddPlayer,

    /// Player joined into a player slot, but is spectating.
    AddSpectator,

    /// Player joined into a spectator slot.
    AddPureSpectator,

    /// Player stopped spectating.
    MoveToPlayer,

    /// A player started spectating, but still has their player slot.
    MoveToSpectator,

    /// A player has moved into a spectator slot.
    MoveToPureSpectator,

    /// A player with a player slot has disconnected.
    RemovePlayer,

    /// A spectator with a player slot has disconnected.
    RemoveSpectator,

    /// A spectator without a player slot has disconnected.
    RemovePureSpectator,
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

/// A change of the map queue.
#[derive(Debug)]
pub struct QueueDiff {
    /// The first index in the map queue that has changed.
    /// Front entries at lower indexes than this value remain unchanged.
    pub first_changed_idx: usize,
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
    /// The player's formatted display name.
    pub player_display_name: DisplayString,

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

    /// The player's map rank after this run, which may have improved
    /// if `new_record` is `Some`.
    pub new_pos: usize,

    /// If `prev_pos` is `Some`, this is `prev_pos - new_pos`.
    /// Otherwise, this is the total number of records minus `new_pos`.
    pub pos_gained: usize,

    /// `Some` if the player improved their personal best.
    pub new_record: Option<Record>,
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
}

/// A change made to the controller config.
#[derive(Debug)]
pub enum ConfigDiff {
    /// The settings that determine the time limit of each map
    /// have changed.
    NewTimeLimit {
        time_limit_factor: u32,
        time_limit_max_secs: u32,
        time_limit_min_secs: u32,
    },

    /// The duration of the outro after a race has changed.
    NewOutroDuration { secs: u32 },
}
