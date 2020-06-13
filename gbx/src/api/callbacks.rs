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

    /// Sent when the countdown is over, and the player
    /// can accelerate.
    ///
    /// Triggered by `Trackmania.Event.StartLine`
    RunStartline { player_login: String },

    /// Sent when a player crosses a checkpoint,
    /// or the finish line.
    ///
    /// Triggered by `Trackmania.Event.WayPoint`
    RunCheckpoint { event: CheckpointEvent },

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
    PlayerAnswer {
        from_uid: i32,
        from_login: String,
        answer: String,
    },

    /// Sent when either the playlist or playlist indexes changed.
    PlaylistChanged {
        curr_idx: Option<i32>,
        next_idx: i32,
    },
}
