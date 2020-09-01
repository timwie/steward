use crate::api::*;

/// Remote procedure calls to be executed on controller-side.
///
/// References:
///  - https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-callbacks
///  - https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md
#[derive(Debug)]
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

    /// Sent when a map is being loaded, or restarted.
    ///
    /// Triggered by `Maniaplanet.LoadingMap_Start`
    MapLoad { is_restart: bool },

    /// Sent when the race ends - `S_TimeLimit` seconds after `RaceBegin`.
    ///
    /// Triggered by `ManiaPlanet.EndMatch`
    RaceEnd,

    /// Sent when the map is unloaded - `S_ChatTime` seconds  after `RaceEnd`.
    /// A map is not unloaded when restarted.
    ///
    /// Triggered by `Maniaplanet.UnloadingMap_Start`
    MapUnload,

    /// Sent alongside `RaceEnd` and `MapUnload`.
    ///
    /// Triggered by `Trackmania.Scores`
    ///
    /// Can also be triggered on demand with `Calls::request_scores`
    MapScores { scores: Scores },

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
}
