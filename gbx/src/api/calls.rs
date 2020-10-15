use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use crate::api::structs::*;
use crate::Fault;

pub(in crate) type Result<T> = std::result::Result<T, Fault>;

/// Server and script method calls.
///
/// These are remote procedure calls executed on the game server.
#[async_trait]
pub trait Calls {
    /// Fetch some info about the server version, which can help
    /// identifying possible incompatibilities.
    ///
    /// Calls method:
    ///     GetVersion
    async fn server_build_info(&self) -> ServerBuildInfo;

    /// Fetch the server's network stats.
    ///
    /// Calls method:
    ///     GetNetworkStats
    async fn server_net_stats(&self) -> ServerNetStats;

    /// Fetch the active server options.
    ///
    /// Initial values are defined in the `<server_options>` section
    /// in the config file (located in `/UserData/Config`).
    ///
    /// Calls method:
    ///     GetServerOptions
    async fn server_options(&self) -> ServerOptions;

    /// Overwrite the server options found in the `<server_options>` section
    /// in the config file (located in `/UserData/Config`).
    ///
    /// Calls method:
    ///     SetServerOptions
    async fn set_server_options(&self, options: &ServerOptions);

    /// Fetch some info about the running mode script, which can help
    /// identifying possible incompatibilities.
    ///
    /// Calls method:
    ///     GetModeScriptInfo
    async fn mode(&self) -> ModeInfo;

    /// Set the mode script.
    ///
    /// Requires a map skip/restart to be take effect.
    ///
    /// Faults for custom modes if there is no script with the given file name.
    ///
    /// Calls method:
    ///     SetScriptName
    async fn set_mode(&self, script: ModeScript) -> Result<()>;

    /// Fetch the values of some settings of the Time Attack mode.
    /// The initial values are found in the match settings in
    /// `/UserData/Maps/MatchSettings/*.txt`.
    ///
    /// Calls method:
    ///     GetModeScriptSettings
    async fn mode_options(&self) -> ModeOptions;

    /// Overwrite game mode settings, that default to the match
    /// settings in `/UserData/Maps/MatchSettings/*.txt`.
    ///
    /// Faults when the given options are for a different game mode.
    ///
    /// Calls method:
    ///     SetModeScriptSettings
    async fn set_mode_options(&self, options: &ModeOptions) -> Result<()>;

    /// Fetch the current match/map/round scores.
    ///
    /// Calls script method:
    ///     Trackmania.GetScores
    ///
    /// Triggers script callback:
    ///     Trackmania.Scores
    async fn scores(&self) -> Scores;

    /// Set the score of a player, and return the new scores.
    ///
    /// Every `Points` field that is `Some` will override the respective number of points.
    /// Different game modes will use different types of points.
    ///
    /// This call never faults. It also does not trigger any callbacks, so in order to
    /// check whether the call was successful, you would want to compare check the player's
    /// new score in the returned scores.
    ///
    /// Calls script method:
    ///     Trackmania.SetPlayerPoints
    ///
    /// Triggers script callback:
    ///     Trackmania.Scores
    async fn set_player_score(&self, login: &str, points: Points) -> Scores;

    /// Set the score of a team, and return the new scores.
    ///
    /// Every `Points` field that is `Some` will override the respective number of points.
    /// Different game modes will use different types of points.
    ///
    /// This call never faults. It also does not trigger any callbacks, so in order to
    /// check whether the call was successful, you would want to compare check the team's
    /// new score in the returned scores.
    ///
    /// Calls script method:
    ///     Trackmania.SetTeamPoints
    ///
    /// Triggers script callback:
    ///     Trackmania.Scores
    async fn set_team_score(&self, team: TeamId, points: Points) -> Scores;

    /// Check whether pauses are supported by the game mode, and if so,
    /// whether there is currently a pause.
    ///
    /// Calls script methods:
    ///     - Maniaplanet.Pause.GetStatus
    ///
    /// Triggers script callback:
    ///     - Maniaplanet.Pause.Status
    async fn pause_status(&self) -> PauseStatus;

    /// Check whether warmups are supported by the game mode, and if so,
    /// whether there is currently a warmup.
    ///
    /// Calls script methods:
    ///     - Trackmania.WarmUp.GetStatus
    ///
    /// Triggers script callback:
    ///     - Trackmania.WarmUp.Status
    async fn warmup_status(&self) -> WarmupStatus;

    /// Fetch the absolute path of the server's `UserData` directory.
    ///
    /// Calls method:
    ///     GameDataDirectory
    async fn user_data_dir(&self) -> PathBuf;

    /// Get the list of connected players.
    ///
    /// Calls method:
    ///     GetPlayerList
    async fn players(&self) -> Vec<PlayerInfo>;

    /// Fetch information about the map with the given file name.
    ///
    /// Faults if there is no map with that file name.
    ///
    /// Calls method:
    ///     GetMapInfo
    async fn map(&self, file_name: &str) -> Result<MapInfo>;

    /// Fetch the playlist.
    ///
    /// Calls method:
    ///     GetMapList
    async fn playlist(&self) -> Vec<PlaylistMap>;

    /// Fetch the current playlist index, or `None` if the current map is
    /// no longer in the playlist.
    ///
    /// Calls method:
    ///     GetCurrentMapIndex
    async fn playlist_current_index(&self) -> Option<usize>;

    /// Fetch the playlist index of the map that will be played once
    /// the current map ends.
    ///
    /// Calls method:
    ///     SetNextMapIndex
    async fn playlist_next_index(&self) -> usize;

    /// Append the map at the specified file name to the end of the playlist.
    ///
    /// Faults if the map was already added.
    ///
    /// Calls method:
    ///     AddMap
    async fn playlist_add(&self, map_file_name: &str) -> Result<()>;

    /// Append the maps at the specified file names to the playlist.
    /// Maps that are already in the playlist will not be duplicated.
    ///
    /// Calls method:
    ///     AddMapList
    async fn playlist_add_all(&self, map_file_names: Vec<&str>);

    /// Remove the map at the specified file name from the playlist.
    ///
    /// Faults if this map is not part of the playlist.
    ///
    /// Calls method:
    ///     RemoveMap
    async fn playlist_remove(&self, map_file_name: &str) -> Result<()>;

    /// Replace the entire playlist with the maps at the specified file names.
    ///
    /// Calls methods:
    /// - GetMapList
    /// - RemoveMapList
    /// - AddMapList
    async fn playlist_replace(&self, map_file_names: Vec<&str>);

    /// Queue the map at the specified playlist index.
    ///
    /// This map will be played after the current concludes.
    /// A successful restart call vote will still replay the current map though.
    ///
    /// Faults if the specified index is the same as the current one,
    /// or doesn't exist, which means this function cannot be used
    /// to restart a map.
    ///
    /// Calls method:
    ///     SetNextMapIndex
    async fn playlist_change_next(&self, map_index: i32) -> Result<()>;

    /// Load the mode, mode settings, and playlist in
    /// `UserData/Maps/MatchSettings/<file_name>.txt`.
    ///
    /// Faults if the given file does not exist, is invalid,
    /// or if not a single map file in the playlist exists
    ///
    /// If the match settings are for a different mode,
    /// the current map has to be unloaded for the new mode to be played.
    ///
    /// All current script settings are immediately overridden
    /// if they exist in the given match settings, even if the modes don't match.
    ///
    /// Calls method:
    ///     LoadMatchSettings
    async fn load_match_settings(&self, file_name: &str) -> Result<()>;

    /// Save the mode, mode settings, and playlist in
    /// `UserData/Maps/MatchSettings/<file_name>.txt`.
    ///
    /// Faults if the file name is not valid.
    ///
    /// Calls method:
    ///     SaveMatchSettings
    async fn save_match_settings(&self, file_name: &str) -> Result<()>;

    /// Send chat message to all players. This message will have no sender.
    ///
    /// Calls method:
    ///     ChatSendServerMessage
    async fn chat_send(&self, msg: &str);

    /// Send a chat message to the specified player logins. This message will have no sender.
    ///
    /// Faults if the player is no longer connected.
    ///
    /// Calls method:
    ///     ChatSendServerMessageToLogin
    async fn chat_send_to(&self, msg: &str, logins: Vec<&str>) -> Result<()>;

    /// Send a chat message to the specified player login, on behalf of a sender login.
    ///
    /// Empty `logins` will send the message to all players.
    ///
    /// Calls method:
    ///     ChatForwardToLogin
    async fn chat_send_from_to(&self, msg: &str, from: &str, logins: Vec<&str>) -> Result<()>;

    /// Send a Manialink to all connected players.
    ///
    /// To remove a single Manialink, send an empty one
    /// with the same ID (`<manialink id="...">`).
    ///
    /// Calls method:
    ///     SendDisplayManialinkPage
    async fn send_manialink(&self, ml: &str);

    /// Send a Manialink to the specified player.
    ///
    /// Faults if the player is no longer connected.
    ///
    /// To remove a single Manialink, send an empty one
    /// with the same ID (`<manialink id="...">`).
    ///
    /// Calls method:
    ///     SendDisplayManialinkPageToId
    async fn send_manialink_to(&self, ml: &str, player_uid: i32) -> Result<()>;

    /// Moves a player to spectator, and removes their player slot,
    /// effectively making place for another player.
    ///
    /// Faults if the player is no longer connected.
    ///
    /// Calls methods:
    /// - ForceSpectatorId(*, 3)
    /// - SpectatorReleasePlayerSlotId
    async fn force_pure_spectator(&self, player_uid: i32) -> Result<()>;

    /// Blacklist the player with the specified login.
    ///
    /// Faults if that player is already blacklisted.
    ///
    /// Calls method:
    ///     BlackList
    async fn blacklist_add(&self, player_login: &str) -> Result<()>;

    /// Remove the specified player from the blacklist.
    ///
    /// Faults if that player is not blacklisted.
    ///
    /// Calls method:
    ///     UnBlackList
    async fn blacklist_remove(&self, player_login: &str) -> Result<()>;

    /// Fetch the list of blacklisted players.
    ///
    /// Calls method:
    ///     GetBlackList
    async fn blacklist(&self) -> Vec<String>;

    /// Load the blacklist file with the specified file name in
    /// the `/UserData/Config/` directory.
    ///
    /// Faults if the specified file is not valid or does not exist.
    ///
    /// Calls method:
    ///     LoadBlackList
    async fn blacklist_load(&self, file_name: &str) -> Result<()>;

    /// Save the blacklist in the file with specified file name in
    /// the `/UserData/Config/` directory.
    ///
    /// Faults if the specified path is not valid or the file
    /// could not be written.
    ///
    /// Calls method:
    ///     SaveBlackList
    async fn blacklist_save(&self, file_name: &str) -> Result<()>;

    /// Clear the blacklist at the server and in the given file.
    ///
    /// Faults if the specified path is not valid or the file
    /// could not be written.
    ///
    /// Calls methods:
    /// - CleanBlackList
    /// - SaveBlackList
    async fn blacklist_clear(&self, file_name: &str) -> Result<()>;

    /// Kick the player with the specified login, with an optional message.
    ///
    /// Faults if no such player is connected.
    ///
    /// Calls method:
    ///     Kick
    async fn kick_player(&self, login: &str, reason: Option<&str>) -> Result<()>;

    /// Quit the server application.
    ///
    /// Calls methods:
    ///     - StopServer
    ///     - QuitGame
    async fn shutdown_server(&self);
}

/// Server and script method calls that are typically used right after
/// establishing a connection to the game server.
#[async_trait]
pub trait SetupCalls {
    /// Allows authentication by specifying a login and a password,
    /// to gain access to the set of functionality corresponding to this
    /// authorization level. This controller should have "SuperAdmin"
    /// privileges.
    ///
    /// This function should be called right after establishing a connection
    /// to the game server, to ensure that other calls will work.
    ///
    /// Calls method:
    ///     Authenticate
    async fn authenticate(&self, username: &str, password: &str);

    /// Has to be called in order to receive callbacks.
    ///
    /// This function should be called right after establishing a connection
    /// to the game server, to ensure that other calls will work.
    ///
    /// Calls method:
    ///     EnableCallbacks(true)
    ///
    /// Calls script method:
    ///     XmlRpc.EnableCallbacks(true)
    async fn enable_callbacks(&self);

    /// Instructs the game server to use the supported API version.
    /// Changes callback and structure names, removes deprecated methods etc.
    ///
    /// This function should be called right after establishing a connection
    /// to the game server, to ensure that other calls will work.
    ///
    /// Has potential to fault, since not every mode implements XmlRpc.SetApiVersion.
    ///
    /// Calls method:
    ///     SetApiVersion
    ///
    /// Calls script method:
    ///     XmlRpc.SetApiVersion
    async fn set_api_version(&self);

    /// Instructs the game server when to send checkpoint times for races and laps.
    ///
    /// The checkpoint times in the `Trackmania.Event.WayPoint` data will now be included
    /// at the end of races and laps.
    ///
    /// Calls script methods:
    /// - Trackmania.Event.SetCurRaceCheckpointsMode
    /// - Trackmania.Event.SetCurLapCheckpointsMode
    async fn set_checkpoint_event_mode(&self);

    /// Chat messages are no longer dispatched to the players, but are instead
    /// routed to `Callback::PlayerChat`. Player messages have to be manually
    /// forwarded with `chat_send_from_to()` for them to appear in the chat.
    /// Messages from the server are still forwarded automatically.
    ///
    /// Faults if the server's chat is already manually routed by another controller.
    ///
    /// Calls method:
    ///     ChatEnableManualRouting(true, true)
    async fn enable_manual_chat_routing(&self) -> Result<()>;

    /// Clear all Manialinks for all players.
    ///
    /// This function can be called right after establishing a connection to
    /// the game server, to remove any leftover Manialinks that were not previously
    /// removed.
    ///
    /// Calls method:
    ///     SendHideManialinkPage
    async fn clear_manialinks(&self);
}

/// Server & script method calls that control the course of game modes.
#[async_trait]
pub trait ModeCalls {
    /// Restart the current map.
    ///
    /// Calls method:
    ///     RestartMap
    async fn restart_map(&self);

    /// Switch to the next map.
    ///
    /// Faults if the map is currently changing.
    ///
    /// Calls method:
    ///     NextMap
    async fn end_map(&self) -> Result<()>;
}

/// Script method calls that control the course of round-based modes.
///
/// The default round-based modes are:
///  - Cup
///  - Champion
///  - Knockout
///  - Rounds
///  - Teams
#[async_trait]
pub trait RoundBasedModeCalls {
    /// Pause the game mode, if it supports pauses.
    ///
    /// Does *not* fault if pauses are not supported by the game mode.
    ///
    /// Calls script method:
    ///     Maniaplanet.Pause.SetActive
    ///
    /// Triggers script callback:
    ///     Maniaplanet.Pause.Status
    async fn pause(&self) -> PauseStatus;

    /// Unpause the game mode, if it supports pauses.
    ///
    /// Calls script method:
    ///     Maniaplanet.Pause.SetActive
    ///
    /// Triggers script callback:
    ///     Maniaplanet.Pause.Status
    async fn unpause(&self) -> PauseStatus;

    /// Stop the warmup sequence, and skip all remaining warmup rounds.
    ///
    /// Does *not* fault if not in warmup.
    ///
    /// Calls script method:
    ///     Trackmania.WarmUp.ForceStop
    async fn force_end_warmup(&self);

    /// Extend the duration of the ongoing warmup round.
    ///
    /// Does *not* fault if not in warmup.
    ///
    /// Calls script method:
    ///     Trackmania.WarmUp.Extend
    async fn warmup_extend(&self, duration: Duration);

    /// Stop the current round.
    ///
    /// Does *not* fault when using in non-rounds based game modes.
    ///
    /// Calls script method:
    ///     Trackmania.ForceEndRound
    async fn force_end_round(&self);
}

/// Script method calls that control the course of the Champion mode.
#[async_trait]
pub trait ChampionCalls {
    /// Stop the current match and start a new one.
    ///
    /// Faults if used in another game mode.
    ///
    /// Calls method:
    ///     SendModeScriptCommands
    ///
    /// Triggers script command:
    ///     Command_StartNewMatch
    async fn start_new_match(&self) -> Result<()>;

    /// Set the current round number.
    ///
    /// Stops the current round if it is a different number than the current one.
    ///
    /// Faults if used in another game mode.
    ///
    /// Calls method:
    ///     SendModeScriptCommands
    ///
    /// Triggers script command:
    ///     Command_StartNewMatch
    async fn start_round_nb(&self, round_nb: i32) -> Result<()>;
}
