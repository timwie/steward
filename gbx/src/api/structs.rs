use std::collections::HashMap;

use regex::Regex;
use serde::export::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use lazy_static::lazy_static;

/// Server version information.
///
/// Reference: GetVersion https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    /// f.e. "ManiaPlanet" - this is not the display name of the server!
    pub name: String,

    /// f.e. "TMStadium@nadeo"
    pub title_id: String,

    /// f.e. "3.3.0"
    pub version: String,

    /// f.e. "2019-10-23_20_00"
    pub build: String,

    /// Setting the API version works, but does not affect
    /// this value it seems, so it might not have any use.
    pub api_version: String,
}

/// Server options that default to the values of the `<dedicated>`
/// config in `.../UserData/Config/*.txt`
///
/// Any `next_*` option will become active as the `current_*`
/// option on map change.
///
/// Reference: GetServerOptions https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ServerOptions {
    /// The server name, as displayed in the server browser.
    ///
    /// Config: `<name>` in `<server_options>`
    pub name: GameString,

    /// The server comment, as displayed in the server browser.
    ///
    /// Config: `<comment>` in `<server_options>`
    pub comment: GameString,

    /// The password needed to connect as a player.
    ///
    /// Config: `<password>` in `<server_options>`
    pub password: String,

    /// The password needed to connect as a spectator.
    ///
    /// Config: `<password_spectator>` in `<server_options>`
    #[serde(rename = "PasswordForSpectator")]
    pub password_spectator: String,

    /// The number of player slots.
    ///
    /// Config: `<max_players>` in `<server_options>`
    pub current_max_players: i32,

    /// see `next_max_players`
    pub next_max_players: i32,

    /// The number of spectator slots.
    ///
    /// Config: `<max_spectators>` in `<server_options>`
    pub current_max_spectators: i32,

    /// see `current_max_spectators`
    pub next_max_spectators: i32,

    /// If true, the server will keep a player's slot when
    /// they switch to spectator.
    ///
    /// Config: `<keep_player_slots>` in `<server_options>`
    pub keep_player_slots: bool,

    /// If true, the server can upload custom player data.
    /// Disable this to save bandwidth, and disk space used
    /// for caching.
    ///
    /// Config: `<enable_p2p_upload>` in `<server_options>`
    #[serde(rename = "IsP2PUpload")]
    pub is_p2p_upload: bool,

    /// If true, the server can download custom player data.
    /// Disable this to save bandwidth, and disk space used
    /// for caching.
    ///
    /// Config: `<enable_p2p_download>` in `<server_options>`
    #[serde(rename = "IsP2PDownload")]
    pub is_p2p_download: bool,

    /// Either "disabled" (0) or "forced" (1).
    ///
    /// Config: `<ladder_mode>` in `<server_options>`
    pub current_ladder_mode: i32,

    /// see `current_ladder_mode`
    pub next_ladder_mode: i32,

    /// If true, players can download maps from the server.
    ///
    /// Config: `<allow_map_download>` in `<server_options>`
    pub allow_map_download: bool,

    /// If true, replays of each map with every player will be auto-saved.
    /// These replays *cannot* be used for validation.
    ///
    /// Config: `<autosave_replays>` in `<server_options>`
    pub auto_save_replays: bool,

    /// If true, validation replays are auto-saved every time a player
    /// completes a run.
    ///
    /// Config: `<autosave_validation_replays>` in `<server_options>`
    pub auto_save_validation_replays: bool,

    /// Either "visible" (0) or "hidden" (1).
    pub hide_server: i32,

    /// If true, introduces a "micro shift" of the car position on the start line,
    /// different on each restart. This hinders recording a run and playing back
    /// the exact same input sequence to repeat the exact same run.
    ///
    /// Not activated by default, since it breaks "press forward" tracks,
    /// which is not an issue for online races.
    ///
    /// Reference: http://www.tm-forum.com/viewtopic.php?p=130016&sid=6b52ce2287e2c4bd9b726973a66c7a0e#p130016
    ///
    /// Config: `<use_changing_validation_seed>` in `<server_options>`
    pub current_use_changing_validation_seed: bool,

    /// see `current_use_changing_validation_seed`
    pub next_use_changing_validation_seed: bool,

    /// If true, player horns are disabled.
    ///
    /// Config: `<disable_horns>` in `<server_options>`
    pub disable_horns: bool,

    /// If true, disables the automatic messages when a player
    /// connects/disconnects from the server.
    ///
    /// Not in config file.
    pub disable_service_announces: bool,

    /// Either "fast" (0) or "high" (1).
    /// If "high", players could see others cars with additional details, like
    /// front wheels turning, heads moving, suspensions, etc.
    /// It increases the network traffic somewhat.
    /// Apparently it doesn't work very well though.
    ///
    /// Reference: http://www.tm-forum.com/viewtopic.php?p=14486#p14486
    ///
    /// Not in config file.
    pub current_vehicle_net_quality: i32,

    /// see `current_vehicle_net_quality`
    pub next_vehicle_net_quality: i32,

    /// Milliseconds until a vote times out, if the vote ratio is not met.
    ///
    /// Config: `<callvote_timeout>` in `<server_options>`
    pub current_call_vote_time_out: i32,

    /// see `current_call_vote_time_out`
    pub next_call_vote_time_out: i32,

    /// The default ratio of players in favour needed for a vote to be
    /// successful. This can be overwritten by more specific `<callvote_ratios>`
    /// settings.
    ///
    /// Config: `<callvote_ratio>` and `<callvote_ratios>` in `<server_options>`
    pub call_vote_ratio: f32,

    /// Password for referees.
    ///
    /// Config: `<referee_password>` in `<server_options>`
    pub referee_password: String,

    /// Either "validate the top3 players" (0) or "validate all players" (1).
    ///
    /// Config: `<referee_validation_mode>` in `<server_options>`
    pub referee_mode: i32,

    /// Only used by ShootMania.
    #[deprecated(since = "0.0.0", note = "Only used by ShootMania")]
    pub client_inputs_max_latency: i32,
}

/// Reference: GetNetworkStats https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct NetStats {
    /// This value might be useful to check that a server has not been online
    /// for more than 30 days. Apparently this can prevent players from joining the server.
    /// (see https://doc.maniaplanet.com/dedicated-server/frequent-errors)
    #[serde(rename = "Uptime")]
    pub uptime_secs: i32,
}

/// Game mode information.
///
/// Reference: GetModeScriptInfo https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ModeInfo {
    /// The name of the game mode script.
    /// For the Time Attack mode, this should be "TimeAttack.Script.txt".
    #[serde(rename = "Name")]
    pub file_name: String,

    /// Comma-delimited; indicates compatible map types for this mode.
    /// For the Time Attack mode, this should be "Race,TrackMania\\Race".
    pub compatible_map_types: String,

    /// Development: The version date of the mode script.
    /// A change in version might be of note.
    pub version: String,
}

/// Game mode options.
///
/// Reference: see result of `ModeInfo.param_descs`, or the mode script itself.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ModeOptions {
    /// S_TimeLimit
    #[serde(rename = "S_TimeLimit")]
    pub time_limit_secs: i32,

    /// S_ChatTime
    #[serde(rename = "S_ChatTime")]
    pub chat_time_secs: i32,
}

/// Player information.
///
/// Reference: SPlayerInfo https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-callbacks
/// Reference: GetPlayerInfo https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, PartialEq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerInfo {
    /// Player UID that is tied to this player while they are connected.
    /// Once they dis- and reconnect, they might have a different UID.
    #[serde(rename = "PlayerId")]
    pub uid: i32,

    /// Player-unique login.
    pub login: String,

    /// Formatted nick name.
    pub nick_name: GameString,

    /// (see functions)
    #[serde(rename = "Flags")]
    flag_digit_mask: i32,

    /// (see functions)
    #[serde(rename = "SpectatorStatus")]
    spectator_digit_mask: i32,
}

impl PlayerInfo {
    /// Signals what slot this player occupies, and whether they are spectating or not:
    /// - `None` if it's not actually a player, but a server login
    /// - `PureSpectator` if a player has only a spectator slot
    /// - `PlayerSpectator` if a player has a player slot, but is spectating
    /// - `Player` if a player has a player slot, and is not spectating
    ///
    /// Note that spectators can only have a player slot if `keep_player_slots` is enabled
    /// in the server options.
    pub fn slot(&self) -> PlayerSlot {
        if !self.is_player() || !self.has_joined() {
            PlayerSlot::None
        } else if !self.has_player_slot() {
            PlayerSlot::PureSpectator
        } else if self.is_spectator() {
            PlayerSlot::PlayerSpectator
        } else {
            PlayerSlot::Player
        }
    }

    /// `True` if the player spectates.
    pub fn is_spectator(&self) -> bool {
        // 2_551_010
        //         ^
        self.spectator_digit_mask % 10 == 1
    }

    /// `True` if the player occupies a player slot.
    pub fn has_player_slot(&self) -> bool {
        // 101_000_000
        //   ^ 1 if has playing slot
        self.flag_digit_mask / 1_000_000 % 10 == 1
    }

    /// `True` if this information belongs to a player.
    ///
    /// One "player" info will actually contain information about
    /// the server, f.e. "login" will be the server's login.
    pub fn is_player(&self) -> bool {
        // 101_000_000
        //     ^ 1 if server
        self.flag_digit_mask / 100_000 % 10 == 0
    }

    /// `False` if the player has disconnected, `True` otherwise.
    pub fn has_joined(&self) -> bool {
        // 101_000_000 yes
        //   1_000_000 no
        self.flag_digit_mask / 100_000_000 % 10 == 1
    }
}

/// Signals what slot a player occupies, and whether they are spectating or not.
#[derive(Debug, Eq, PartialEq)]
pub enum PlayerSlot {
    /// This player has disconnected, or is the "player" that represents
    /// the server.
    None,

    /// Slot for actively racing players that are not spectating.
    Player,

    /// Slot for spectators that still have a player slot.
    /// They can quickly re-join the race, without waiting until a slot frees up.
    PlayerSpectator,

    /// Slot for *pure* spectators, that do not occupy a player slot.
    PureSpectator,
}

impl std::fmt::Debug for PlayerInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerInfo")
            .field("uid", &self.uid)
            .field("login", &self.login)
            .field("slot", &self.slot())
            .field("joined", &self.has_joined())
            .finish()
    }
}

/// Reference: GetMapList https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistMap {
    /// A unique identifier.
    #[serde(rename = "UId")]
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,
}

impl PlaylistMap {
    /// The server's playlist can include bundled maps, that are not stored in `.../UserData/Maps`,
    /// with a file name like this: `Campaigns\0\A01.Map.Gbx`
    pub fn is_campaign_map(&self) -> bool {
        self.file_name.starts_with("Campaigns\\")
    }
}

/// Map information.
///
/// Reference: GetMapInfo https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MapInfo {
    /// A unique identifier.
    #[serde(rename = "UId")]
    pub uid: String,

    /// The formatted map name.
    pub name: GameString,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,

    /// The map author's login.
    #[serde(rename = "Author")]
    pub author_login: String,

    /// The "author time" in milliseconds. This is the time the map
    /// was validated with in the map editor.
    #[serde(rename = "AuthorTime")]
    pub author_millis: i32,
}

/// Run data at the time of crossing any checkpoint or the finish line.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniaeventwaypoint
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct CheckpointEvent {
    /// The driving player's login.
    #[serde(rename = "login")]
    pub player_login: String,

    /// Total duration of the run up to this checkpoint.
    #[serde(rename = "racetime")]
    pub race_time_millis: i32,

    /// The total durations of this run at the time of passing each checkpoint.
    /// The last element will be equal to `race_time_millis`.
    #[serde(rename = "curracecheckpoints")]
    pub race_time_cp_millis: Vec<i32>,

    /// Checkpoint index; or the number of unique checkpoints crossed
    /// since the beginning of this run minus one.
    #[serde(rename = "checkpointinrace")]
    pub cp_index: i32,

    /// `True` if the player has crossed the finish line.
    #[serde(rename = "isendrace")]
    pub is_finish: bool,

    /// Speed of the player in km/h at the time of passing this checkpoint.
    /// This is negative if they are driving backwards!
    pub speed: f32,

    /// Total distance traveled by the player up to this checkpoint.
    pub distance: f32,
}

/// The ranking of the current race.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniascores
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Scores {
    /// Empty, or an ID that was used when explicitly triggering the callback.
    #[serde(rename = "responseid")]
    pub(in crate) response_id: String,

    /// "" | "PreEndRound" | "EndRound" | "EndMap" | "EndMatch"
    pub section: String,

    /// Race ranking sorted from best to worst.
    #[serde(rename = "players")]
    pub entries: Vec<Score>,
}

/// A player's ranking in the current race.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniascores
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Score {
    /// The player's login.
    pub login: String,

    /// The player's formatted nick name.
    #[serde(rename = "name")]
    pub nick_name: GameString,

    /// Rank of the player in the current race.
    #[serde(rename = "rank")]
    pub pos: i32,

    /// Best run time in milliseconds (or -1 if no completed run).
    #[serde(rename = "bestracetime")]
    pub best_time_millis: i32,

    /// Number of respawns during the best run (or -1 if no completed run).
    #[serde(rename = "bestracerespawns")]
    pub best_time_respawns: i32,

    /// Checkpoints times during the best run in milliseconds (or empty if no completed run).
    #[serde(rename = "bestracecheckpoints")]
    pub best_time_cp_millis: Vec<i32>,
}

/// A string with in-game formatting.
#[derive(PartialEq, Clone)]
pub struct GameString {
    /// The formatted string.
    pub formatted: String,
}

impl GameString {
    pub fn from(str: String) -> Self {
        GameString { formatted: str }
    }

    /// Removes all text formatting.
    ///
    /// References:
    /// - https://doc.maniaplanet.com/client/text-formatting
    /// - https://wiki.xaseco.org/wiki/Text_formatting
    pub fn plain(&self) -> String {
        lazy_static! {
            static ref RE_DOLLAR: Regex = Regex::new(r"\${2}").unwrap();
            static ref RE_FORMATTING: Regex =
                Regex::new(r"\$[A-Fa-f0-9]{3}|\$[wWnNoOiItTsSgGzZpP]|\$[lLhHpP]\[.+]").unwrap();
        }

        let output = RE_DOLLAR.replace_all(&self.formatted, r"\$");
        let output = RE_FORMATTING.replace_all(&output, "");
        output.into_owned()
    }
}

impl std::fmt::Debug for GameString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.serialize_str(&self.plain())
    }
}

impl<'de> Deserialize<'de> for GameString {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let formatted: String = serde::de::Deserialize::deserialize(deserializer)?;
        Ok(GameString { formatted })
    }
}

impl Serialize for GameString {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.formatted)
    }
}

/// See `Callback::PlayerAnswered`.
#[derive(Debug)]
pub struct PlayerAnswer {
    /// The answer string, either from a Manialink (`<quad action="my_action"/>`),
    /// or from ManiaScript (`TriggerPageAction("my_action");`)
    pub answer: String,

    /// The current values of Manialink inputs like `<entry name="...">`
    /// or `<textedit name="...">`.
    pub entries: HashMap<String, String>,
}
