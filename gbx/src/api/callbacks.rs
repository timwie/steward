use crate::api::structs::*;

/// Server and mode script callbacks.
///
/// These are remote procedure calls to be executed controller-side.
#[derive(Debug, Clone)]
pub enum Callback {
    /// Sent when player info changes, f.e. when entering or leaving spectator
    /// mode. Sent on connect, but not sent on disconnect.
    ///
    /// Triggered by `ManiaPlanet.PlayerInfoChanged`
    PlayerInfoChanged(PlayerInfo),

    /// Sent when a player disconnects from the server.
    ///
    /// Triggered by `ManiaPlanet.PlayerDisconnect`
    PlayerDisconnect { login: String },

    /// Sent when the countdown is displayed for the player.
    ///
    /// Since TMNext, this callback is *not* followed by `RunStartline`!
    /// Both have to be handled to know when a player is starting a run.
    ///
    /// Triggered by
    /// - `Trackmania.Event.GiveUp`
    /// - `Trackmania.Event.SkipOutro`
    PlayerCountdown { login: String },

    /// Sent when the countdown is over, and the player can accelerate.
    ///
    /// Since TMNext, this callback is *not* preceeded by `RunStartline`!
    /// Both have to be handled to know when a player is starting a run.
    ///
    /// Triggered by `Trackmania.Event.StartLine`
    PlayerStartline { login: String },

    /// Sent when a player crosses a checkpoint, or the finish line.
    ///
    /// Triggered by `Trackmania.Event.WayPoint`
    PlayerCheckpoint(CheckpointEvent),

    /// Sent when a player respawns at the previous checkpoint.
    ///
    /// Triggered by `Trackmania.Event.Respawn`
    PlayerCheckpointRespawn(CheckpointRespawnEvent),

    /// Sent when client & server run times are out of sync. This is likely caused
    /// by connection issues, but could also be a cheating attempt.
    ///
    /// Triggered by `TrackMania.PlayerIncoherence`
    ///
    /// Can also be triggered by `Trackmania.Event.WayPoint`, when `race_time_millis`
    /// is set to zero.
    PlayerIncoherence { login: String },

    /// Sent when a player writes something in the chat.
    ///
    /// Triggered by `ManiaPlanet.PlayerChat`
    PlayerChat {
        from_uid: i32,
        from_login: String,
        message: String,
    },

    /// Sent when
    /// - a player interacts with a Manialink element that defines an action,
    ///   f.e. when clicking `<quad action="my_action"/>`
    /// - a ManiaScript triggers an action with `TriggerPageAction("my_action");`
    ///
    /// Triggered by `ManiaPlanet.PlayerManialinkPageAnswer`
    PlayerAnswered {
        from_uid: i32,
        from_login: String,
        answer: PlayerManialinkEvent,
    },

    /// Sent when either the playlist or playlist indexes changed.
    ///
    /// Triggered by `ManiaPlanet.MapListModified`
    PlaylistChanged {
        curr_idx: Option<i32>,
        next_idx: i32,
        playlist_modified: bool,
    },

    /// Sent when the game mode script enters or exits a certain section.
    ModeScriptSection(ModeScriptSection),

    /// Sent when a warmup round starts.
    ///
    /// Triggered by `Trackmania.WarmUp.StartRound`
    WarmupBegin(WarmupRoundStatus),

    /// Sent when a warmup round ends.
    ///
    /// Triggered by `Trackmania.WarmUp.EndRound`
    WarmupEnd(WarmupRoundStatus),

    /// Sent when a round, map or match ends.
    ///
    /// Triggered by `Trackmania.Scores`, with `Calls::scores`
    Scores(Scores),

    /// Sent at the end of each round in the Champion game mode.
    ///
    /// Triggered by `Trackmania.Champion.Scores`
    ChampionRoundEnd(ChampionEndRoundEvent),

    /// Sent at the end of each round in the Knockout game mode.
    ///
    /// Triggered by `Trackmania.Knockout.Elimination`
    KnockoutRoundEnd(KnockoutEndRoundEvent),

    /// Triggered by `Maniaplanet.Pause.Status` with `Calls::pause_status`,
    /// and by `Maniaplanet.Pause.SetActive` with `Calls::pause` or `Calls::unpause`.
    PauseStatus(PauseStatus),

    /// Triggered by `Trackmania.WarmUp.Status` with `Calls::warmup_status`.
    WarmupStatus(WarmupStatus),
}
