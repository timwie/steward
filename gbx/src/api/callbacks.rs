use crate::api::structs::*;

/// Server and mode script callbacks.
///
/// These are remote procedure calls to be executed controller-side.
#[derive(Debug, Clone)]
pub enum Callback {
    /// Sent when the game mode script reaches a certain section.
    ModeScriptSection(ModeScriptSectionCallback),

    /// Sent during the game mode's playloop.
    Playloop(PlayloopCallback),

    /// Sent when player info changes, f.e. when entering or leaving spectator
    /// mode. Sent on connect, but not sent on disconnect.
    ///
    /// Triggered by `ManiaPlanet.PlayerInfoChanged`
    PlayerInfoChanged(PlayerInfo),

    /// Sent when a player disconnects from the server.
    ///
    /// Triggered by `ManiaPlanet.PlayerDisconnect`
    PlayerDisconnect { login: String },

    /// Sent when either the playlist or playlist indexes changed.
    ///
    /// Triggered by `ManiaPlanet.MapListModified`
    PlaylistChanged {
        curr_idx: Option<i32>,
        next_idx: i32,
        playlist_modified: bool,
    },

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

    /// Triggered by `Trackmania.Scores`, with `Calls::scores`.
    Scores(Scores),

    /// Triggered by `Maniaplanet.Pause.Status` with `Calls::pause_status`,
    /// and by `Maniaplanet.Pause.SetActive` with `Calls::pause` or `Calls::unpause`.
    PauseStatus(PauseStatus),

    /// Triggered by `Trackmania.WarmUp.Status` with `Calls::warmup_status`.
    WarmupStatus(WarmupStatus),
}

/// Lifecycle callbacks at the start or end of certain sections in a game mode script.
///
/// All these callbacks are sent in every mode, unless otherwise stated.
///
/// Unless otherwise stated, the callbacks are sorted chronologically from top to bottom,
/// from early to late in a mode's lifecycle. There are contiguous sequences that may loop -
/// a mode can play multiple matches, a match can have multiple maps, a map can have multiple
/// rounds, and so on.
///
/// # Mode script template
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
/// these callbacks (among others) are represented by this enum. Note that the callbacks
/// will also be triggered if they are irrelevant for a mode: the TimeAttack mode will
/// trigger `_Round` sections f.e., even though there are no rounds in this mode.
#[derive(Debug, Clone)]
pub enum ModeScriptSectionCallback {
    /// Sent on server start, and whenever the mode script changes;
    /// when the mode is about to be initialized.
    ///
    /// Triggered by `Maniaplanet.StartServer_Start`
    PreStartServer {
        restarted_script: bool,
        changed_script: bool,
    },

    /// Sent when the mode was initialized.
    ///
    /// Triggered by `Maniaplanet.StartServer_End`
    PostStartServer,

    /// Sent before a match is about to start.
    ///
    /// Triggered by `Maniaplanet.StartMatch_Start`
    PreStartMatch,

    /// Sent after a match was started.
    ///
    /// Triggered by `Maniaplanet.StartMatch_End`
    PostStartMatch,

    /// Sent when a map is about to be loaded or restarted.
    ///
    /// Triggered by `Maniaplanet.LoadingMap_Start`
    PreLoadMap { is_restart: bool },

    /// Sent when a map was loaded or restarted.
    ///
    /// Triggered by `Maniaplanet.LoadingMap_End`
    PostLoadMap,

    /// Sent before the map intro sequence.
    ///
    /// Triggered by `Maniaplanet.StartMap_Start`
    PreStartMap { nth_map_in_match: i32 },

    /// Sent when a warmup round starts.
    ///
    /// Triggered by `Trackmania.WarmUp.StartRound`
    StartWarmupRound(WarmupRoundStatus),

    /// Sent when a warmup round ends.
    ///
    /// Triggered by `Trackmania.WarmUp.EndRound`
    EndWarmupRound(WarmupRoundStatus),

    /// Sent after the map intro sequence and an optional warmup.
    ///
    /// Triggered by `Maniaplanet.StartMap_End`
    PostStartMap { nth_map_in_match: i32 },

    /// Sent when a round is about to start.
    ///
    /// Triggered by `Maniaplanet.StartRound_Start`
    PreStartRound { nth_round_on_map: i32 },

    /// Sent after a round was started.
    ///
    /// Triggered by `Maniaplanet.StartRound_End`
    PostStartRound { nth_round_on_map: i32 },

    /// Sent when players are about to start playing.
    ///
    /// Triggered by `Maniaplanet.StartPlayLoop`
    StartPlayloop,

    /// Sent when players are done playing.
    ///
    /// Triggered by `Maniaplanet.EndPlayLoop`
    EndPlayloop,

    /// Sent when a round is about to end.
    ///
    /// Triggered by `Maniaplanet.EndRound_Start`
    PreEndRound { nth_round_on_map: i32 },

    /// Sent before round scores are final.
    ///
    /// When there is a request to end the round or match, the scores will reset for
    /// `EndRoundScores`. The scores in this variant are sent before they are reset.
    /// If there is no such request, these scores should be ignored in favor of `EndRoundScores`.
    ///
    /// Triggered by `Trackmania.Scores` with section `PreEndRound`
    PreEndRoundScores(Scores),

    /// Sent when Champion round scores are final.
    ///
    /// Triggered by `Trackmania.Champion.Scores`
    EndRoundChampionScores(ChampionEndRoundEvent),

    /// Sent when players are eliminated at the end of a Knockout round.
    ///
    /// Triggered by `Trackmania.Knockout.Elimination`
    EndRoundKnockoutEliminations(KnockoutEndRoundEvent),

    /// Sent when round scores are final.
    ///
    /// Triggered by `Trackmania.Scores` with section `EndRound`
    EndRoundScores(Scores),

    /// Sent when a round has ended.
    ///
    /// Triggered by `Maniaplanet.EndRound_End`
    PostEndRound { nth_round_on_map: i32 },

    /// Sent when a map is about to end.
    ///
    /// Triggered by `Maniaplanet.EndMap_Start`
    PreEndMap { nth_map_in_match: i32 },

    /// Sent when the map scores are final.
    ///
    /// Triggered by `Trackmania.Scores` with section `EndMap`
    EndMapScores(Scores),

    /// Sent when a map has ended.
    ///
    /// Triggered by `Maniaplanet.EndMap_End`
    PostEndMap { nth_map_in_match: i32 },

    /// Sent before a map is unloaded.
    ///
    /// Triggered by `Maniaplanet.UnloadingMap_Start`
    PreUnloadMap,

    /// Sent after a map was unloaded.
    ///
    /// Triggered by `Maniaplanet.UnloadingMap_End`
    PostUnloadMap,

    /// Sent when a match is about to end.
    ///
    /// Triggered by `Maniaplanet.EndMatch_Start`
    PreEndMatch,

    /// Sent when the match scores are final.
    ///
    /// For the Champion and Knockout modes, this is already sent during the podium sequence,
    /// before the map is unloaded.
    ///
    /// Triggered by `Trackmania.Scores` with section `EndMatch`
    EndMatchScores(Scores),

    /// Sent when a match has ended.
    ///
    /// Triggered by `Maniaplanet.EndMatch_End`
    PostEndMatch,

    /// Sent when the next match will not be played in the same mode.
    ///
    /// Triggered by `Maniaplanet.EndServer_Start`
    PreEndServer,

    /// Sent when the the mode script is about to terminate, and a new mode
    /// is entered.
    ///
    /// Triggered by `Maniaplanet.EndServer_End`
    PostEndServer,
}

/// Callbacks sent during a mode script's playloop section.
///
/// These callbacks are only sent when there are players driving.
#[derive(Debug, Clone)]
pub enum PlayloopCallback {
    /// Sent when the countdown is over, and the player can accelerate.
    ///
    /// In the TimeAttack mode, a player skipping the outro will result in this callback
    /// *not* being sent when at the start line. `SkipOutro` has to be handled as well to
    /// know when a player is starting a run.
    ///
    /// Triggered by `Trackmania.Event.StartLine`
    StartLine { login: String },

    /// Sent when a player crosses a checkpoint, or the finish line.
    ///
    /// Triggered by `Trackmania.Event.WayPoint`
    Checkpoint(CheckpointEvent),

    /// Sent when a player respawns at the previous checkpoint.
    ///
    /// Triggered by `Trackmania.Event.Respawn`
    CheckpointRespawn(CheckpointRespawnEvent),

    /// Sent when a player gives up.
    ///
    /// In the TimeAttack mode, the player will respawn at the start line.
    ///
    /// Triggered by `Trackmania.Event.GiveUp`
    GiveUp { login: String },

    /// Sent when a player skipped the outro.
    ///
    /// In the TimeAttack mode, "skipping the outro" means that the player
    /// did not wait to be respawned after finishing, but pressed the respawn
    /// button to skip the wait.
    ///
    /// In the TimeAttack mode, this callback is *not* followed by `Startline`!
    /// Both have to be handled to know when a player is starting a run.
    ///
    /// Triggered by `Trackmania.Event.SkipOutro`
    SkipOutro { login: String },

    /// Sent when client & server run times are out of sync.
    ///
    /// This is likely caused by connection issues, but could also be a cheating attempt.
    ///
    /// Triggered by `TrackMania.PlayerIncoherence`, or by `Trackmania.Event.WayPoint`
    /// when `race_time_millis` is set to zero.
    Incoherence { login: String },
}
