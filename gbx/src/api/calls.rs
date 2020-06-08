use std::path::PathBuf;

use async_trait::async_trait;

use crate::api::*;
use crate::Fault;

pub(in crate) type Result<T> = std::result::Result<T, Fault>;

/// Remote procedure calls on the game server.
///
/// Every call might panic if the connection to the game server was interrupted.
///
/// References:
///  - https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
///  - https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md
#[async_trait]
pub trait Calls: Send + Sync {
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
    /// Calls methods:
    /// - EnableCallbacks(true)
    /// - XmlRpc.EnableCallbacks(true)
    async fn enable_callbacks(&self);

    /// Instructs the game server to use the supported API version.
    /// Changes callback and structure names, removes deprecated methods etc.
    ///
    /// This function should be called right after establishing a connection
    /// to the game server, to ensure that other calls will work.
    ///
    /// Calls methods:
    /// - SetApiVersion
    /// - XmlRpc.SetApiVersion
    ///
    /// Potential error source: not every mode implements XmlRpc.SetApiVersion.
    async fn set_api_version(&self);

    /// Chat messages are no longer dispatched to the players, but are instead
    /// routed to `Callbacks.on_player_chat()`. The messages have to be manually
    /// forwarded with `chat_send_from_to()` for them to appear in the chat.
    /// Messages from the server are still forwarded automatically.
    ///
    /// Calls method:
    ///     ChatEnableManualRouting(true, true)
    async fn enable_manual_chat_routing(&self);

    /// Hides all Manialink pages for all players.
    ///
    /// This function can be called right after establishing a connection to
    /// the game server, to remove any leftover Manialinks that were not previously
    /// removed.
    ///
    /// Calls method:
    ///     SendHideManialinkPage
    async fn clear_manialinks(&self);

    /// Fetch some info about the server version, which can help
    /// identifying possible incompatibilities.
    ///
    /// Calls method:
    ///     GetVersion
    async fn server_info(&self) -> ServerInfo;

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
    /// Faults in edge cases with message "No current script",
    /// f.e. when `set_mode` failed.
    ///
    /// Calls method:
    ///     GetModeScriptInfo
    async fn mode(&self) -> ModeInfo;

    /// Set the mode script and restart.
    ///
    /// Faults if the script contains syntax errors.
    ///
    /// Calls method:
    ///     SetModeScriptText
    async fn set_mode(&self, script_text: &str) -> Result<()>;

    /// Fetch the absolute path of the server's `UserData` directory.
    ///
    /// Calls method:
    ///     GameDataDirectory
    async fn user_data_dir(&self) -> PathBuf;

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
    /// Calls method:
    ///     SetModeScriptSettings
    async fn set_mode_options(&self, options: &ModeOptions);

    /// Set the `<ui_properties>` of the server. We can hide many overlays
    /// with ManiaScript, but we need this method for the offset positions
    /// in particular.
    ///
    /// Probably faults on a non-Trackmania game modes.
    ///
    /// Faults if the XML document is not valid.
    ///
    /// Calls method:
    ///     Trackmania.UI.SetProperties
    // Use a raw string, since this is using neither XML-RPC structs,
    // nor JSON, but some custom XML tags. Not worth the effort
    // to serialize this from some struct.
    async fn set_ui_properties(&self, xml: &str);

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
    async fn playlist(&self) -> Vec<MapInfo>;

    /// Fetch the current playlist index, or `None` if the current map is
    /// no longer in the playlist.
    ///
    /// Calls method:
    ///     GetCurrentMapIndex
    async fn playlist_current_index(&self) -> Option<usize>;

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

    /// Replace the playlist inside `UserData/Maps/MatchSettings/<file_name>.txt`
    /// with the current playlist.
    ///
    /// Faults if the file name is not valid.
    ///
    /// Calls method:
    ///     SaveMatchSettings
    async fn playlist_save(&self, file_name: &str);

    /// Queue the map at the specified playlist index.
    /// This map will be played after the current concludes.
    /// A successful restart vote will still replay the current map though.
    ///
    /// Faults if the specified index is the same as the current one,
    /// or doesn't exist, which means this function cannot be used
    /// to restart a map.
    ///
    /// Calls method:
    ///     SetNextMapIndex
    async fn playlist_change_next(&self, map_index: i32) -> Result<()>;

    /// Immediately jump to the map at the specified playlist index.
    ///
    /// Faults if the index doesn't exist.
    ///
    /// Calls method:
    ///     JumpToMapIndex
    async fn playlist_change_current(&self, map_index: i32) -> Result<()>;

    /// Restart the current map.
    ///
    /// This call will make sure that the current map is not unloaded, which means
    /// the `MapEnd` callback will be skipped.
    ///
    /// Calls method:
    ///     RestartMap
    async fn restart_map(&self);

    /// Switch to the next map.
    ///
    /// This call is equivalent to setting the remaining time limit to zero.
    /// All end-of-race callbacks will be invoked, and the chat time will
    /// last the usual amount of time.
    ///
    /// Calls method:
    ///     NextMap
    async fn end_map(&self);

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

    /// Fetch the validation replay of the player's best run on the current map.
    ///
    /// Calls method:
    ///     GetValidationReplay
    async fn validation_replay(&self, player_login: &str) -> Result<Vec<u8>>;

    /// Fetch the ghost replay of the player's best run on the current map.
    ///
    /// Calls method:
    ///     SaveBestGhostsReplay
    async fn ghost_replay(&self, player_login: &str) -> Result<std::io::Result<Vec<u8>>>;

    /// Moves a player to spectator, and removes their player slot,
    /// effectively making place for another player.
    ///
    /// Faults if the player is no longer connected.
    ///
    /// Calls methods:
    /// - ForceSpectatorId(*, 3)
    /// - SpectatorReleasePlayerSlotId
    async fn force_pure_spectator(&self, player_uid: i32) -> Result<()>;

    /// Triggers sending of the `Callback::Scores` callback,
    /// which is otherwise called once a map finishes and scores
    /// are final.
    ///
    /// Probably faults on a non-Trackmania game modes.
    ///
    /// Calls script method:
    ///     Trackmania.GetScores
    ///
    /// Triggers script callback:
    ///     Trackmania.Scores
    async fn request_scores(&self);

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
    async fn load_blacklist(&self, file_name: &str) -> Result<()>;

    /// Save the blacklist in the file with specified file name in
    /// the `/UserData/Config/` directory.
    ///
    /// Faults if the specified path is not valid or the file
    /// could not be written.
    ///
    /// Calls method:
    ///     SaveBlackList
    async fn save_blacklist(&self, file_name: &str) -> Result<()>;

    /// Kick the player with the specified login, with an optional message.
    ///
    /// Faults if no such player is connected.
    ///
    /// Calls method:
    ///     Kick
    async fn kick_player(&self, login: &str, reason: Option<&str>) -> Result<()>;

    /// Fetch the server's network stats.
    ///
    /// Calls method:
    ///     GetNetworkStats
    async fn net_stats(&self) -> NetStats;

    /// Quit the server application.
    ///
    /// Calls methods:
    ///     - StopServer
    ///     - QuitGame
    async fn stop_server(&self);
}
