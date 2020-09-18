use crate::api::structs::*;

/// Remote procedure calls to be executed on controller-side.
///
/// References:
///  - https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-callbacks
///  - https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md
#[derive(Debug, Clone)]
pub enum Callback {
    /// Sent when player info changes, f.e. when entering or leaving spectator
    /// mode. Sent on connect, but not sent on disconnect.
    ///
    /// Triggered by `ManiaPlanet.PlayerInfoChanged`
    PlayerInfoChanged { info: PlayerInfo },

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
    RunCountdown { player_login: String },

    /// Sent when the countdown is over, and the player can accelerate.
    ///
    /// Since TMNext, this callback is *not* preceeded by `RunStartline`!
    /// Both have to be handled to know when a player is starting a run.
    ///
    /// Triggered by `Trackmania.Event.StartLine`
    RunStartline { player_login: String },

    /// Sent when a player crosses a checkpoint, or the finish line.
    ///
    /// Triggered by `Trackmania.Event.WayPoint`
    RunCheckpoint { event: CheckpointEvent },

    /// Sent when client & server run times are out of sync. This is likely caused
    /// by connection issues, but could also be a cheating attempt.
    ///
    /// Triggered by `TrackMania.PlayerIncoherence`
    ///
    /// Can also be triggered by `Trackmania.Event.WayPoint`, when `race_time_millis`
    /// is set to zero.
    RunIncoherence { player_login: String },

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
        answer: PlayerAnswer,
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
    Scores { scores: Scores },

    /// Triggered by `Maniaplanet.Pause.Status` with `Calls::pause_status`,
    /// and by `Maniaplanet.Pause.SetActive` with `Calls::pause` or `Calls::unpause`.
    PauseStatus(WarmupOrPauseStatus),

    /// Triggered by `Trackmania.WarmUp.Status` with `Calls::warmup_status`.
    WarmupStatus(WarmupOrPauseStatus),
}

/// All game modes build on a template (`Libs/Nadeo/TMxSM/Race/ModeTrackmania.Script.txt`)
/// using a structure with several nested loops representing the progression of the game mode.
///
/// Server -> Match -> Map -> Round -> Turn -> PlayLoop
///
/// The server can launch a match.
/// This match can be played on several maps.
/// Each map can be divided into several rounds.
/// Each round can be further divided into several turns.
///
/// The playloop is executed repeatedly until an upper level section
/// (turn, round, map, match or server) is requested to stop.
///
/// The template has several plugs for each loop at the beginning and the end, which
/// allow game modes to implement their logic.
///
/// The template also triggers callbacks when entering or leaving one of the loops;
/// these callbacks are represented by this enum.
#[derive(Debug, Clone)]
pub enum ModeScriptSection {
    PreStartServer {
        restarted_script: bool,
        changed_script: bool,
    },
    PostStartServer,

    PreStartMatch,
    PostStartMatch,

    PreLoadMap {
        is_restart: bool,
    },
    PostLoadMap,

    PreStartMap,
    PostStartMap,

    PreStartRound,
    PostStartRound,

    PrePlayloop,
    PostPlayloop,

    PreEndRound,
    PostEndRound,

    PreEndMap,
    PostEndMap,

    PreUnloadMap,
    PostUnloadMap,

    PreEndMatch,
    PostEndMatch,

    PreEndServer,
    PostEndServer,
}
