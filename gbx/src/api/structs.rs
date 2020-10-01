use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;
use regex::Regex;
use serde::export::Formatter;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Server version information.
///
/// Reference: GetVersion https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    /// "Trackmania" - this is not the display name of the server!
    pub name: String,

    /// f.e. "3.3.0"
    pub version: String,

    /// f.e. "2020-07-01_14_30"
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
    pub name: DisplayString,

    /// The server comment, as displayed in the server browser.
    ///
    /// Config: `<comment>` in `<server_options>`
    pub comment: DisplayString,

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

    /// Leave at 0, which gives "automatic adjustment".
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
    /// The script that implements this mode.
    #[serde(rename = "Name")]
    pub script: ModeScript,

    /// All compatible map types for this mode.
    #[serde(deserialize_with = "deserialize_map_types")]
    pub compatible_map_types: HashSet<MapType>,

    /// Development: The version date of the mode script.
    /// A change in version might be of note.
    pub version: String,
}

/// Map types are scripts that set certain requirements for a map.
/// The default `Race` type f.e. requires exactly one start block and at
/// least one finish block.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MapType {
    /// The default Trackmania map type that is compatible with all default game modes.
    Race,
}

impl From<&str> for MapType {
    fn from(name: &str) -> Self {
        use MapType::*;

        // "TrackMania\\TM_Race" or just "TM_Race" is the only default map type,
        // and is supported by all default modes.

        match name {
            "TrackMania\\TM_Race" => Race,
            "TM_Race" => Race,
            _ => panic!("custom map types are not supported"),
        }
    }
}

fn deserialize_map_types<'de, D>(deserializer: D) -> Result<HashSet<MapType>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let comma_delimited: String = serde::de::Deserialize::deserialize(deserializer)?;
    Ok(comma_delimited.split(',').map(MapType::from).collect())
}

/// Game modes that the server can play.
#[derive(Debug, PartialEq, Clone)]
pub enum ModeScript {
    Champion, // round-based
    Cup,      // round-based
    Knockout, // round-based
    Laps,
    Rounds, // round-based
    Teams,  // round-based
    TimeAttack,
    Other {
        /// The relative script file name in `/UserData/Scripts/Modes`.
        file_name: String,
    },
}

impl ModeScript {
    pub fn file_name(&self) -> &str {
        use ModeScript::*;

        match self {
            Champion => "Trackmania/TM_Champion_Online.Script.txt",
            Cup => "Trackmania/TM_Cup_Online.Script.txt",
            Knockout => "Trackmania/TM_Knockout_Online.Script.txt",
            Laps => "Trackmania/TM_Laps_Online.Script.txt",
            Rounds => "Trackmania/TM_Rounds_Online.Script.txt",
            Teams => "Trackmania/TM_Teams_Online.Script.txt",
            TimeAttack => "Trackmania/TM_TimeAttack_Online.Script.txt",
            Other { file_name } => file_name,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ModeScript::Other { file_name } => file_name
                .trim_start_matches("Trackmania/")
                .trim_end_matches(".Script.txt"),
            _ => self
                .file_name()
                .trim_start_matches("Trackmania/TM_")
                .trim_end_matches("_Online.Script.txt"),
        }
    }

    /// All game modes that come with the dedicated server, and do not have to be
    /// added to the `/UserData/Scripts/Modes` directory.
    pub fn default_modes() -> Vec<ModeScript> {
        use ModeScript::*;
        vec![Champion, Cup, Knockout, Laps, Rounds, Teams, TimeAttack]
    }
}

impl<'de> Deserialize<'de> for ModeScript {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let file_name: String = serde::de::Deserialize::deserialize(deserializer)?;

        let default_mode = ModeScript::default_modes()
            .into_iter()
            .find(|mode| mode.file_name() == file_name);

        Ok(match default_mode {
            Some(mode) => mode,
            None => ModeScript::Other { file_name },
        })
    }
}

/// Game mode settings.
///
/// Every mode script has a different set of possible settings.
#[derive(Debug)]
pub enum ModeOptions {
    Champion(ChampionOptions),
    Cup(CupOptions),
    Knockout(KnockoutOptions),
    Laps(LapsOptions),
    Rounds(RoundsOptions),
    Teams(TeamsOptions),
    TimeAttack(TimeAttackOptions),
}

impl ModeOptions {
    pub fn script(&self) -> ModeScript {
        match self {
            ModeOptions::Champion(_) => ModeScript::Champion,
            ModeOptions::Cup(_) => ModeScript::Cup,
            ModeOptions::Knockout(_) => ModeScript::Knockout,
            ModeOptions::Laps(_) => ModeScript::Laps,
            ModeOptions::Rounds(_) => ModeScript::Rounds,
            ModeOptions::Teams(_) => ModeScript::Teams,
            ModeOptions::TimeAttack(_) => ModeScript::TimeAttack,
        }
    }
}

/// Setting for the TimeAttack game mode.
///
/// References:
/// - Libs/Nadeo/ModeLibs/Common/ModeBase.Script.txt (2020-06-23)
/// - Libs/Nadeo/ModeLibs/Common/ModeMatchmaking.Script.txt (2020-06-09)
/// - Libs/Nadeo/TMxSM/Race/ModeBase.Script.txt (2020-08-25)
/// - Modes/TrackMania/TM_TimeAttack_Online.Script.txt (2020-09-10)
/// - https://doc.maniaplanet.com/dedicated-server/references/settings-list-for-nadeo-gamemodes
#[derive(Serialize, Deserialize, Debug)]
pub struct TimeAttackOptions {
    /// Chat time at the end of a map or match in seconds.
    #[serde(rename = "S_ChatTime")]
    pub chat_time_secs: i32,

    /// Forced number of laps.
    ///
    /// Set to -1 to use laps from map validation.
    /// Set to 0 to use "independent" laps (default in TimeAttack).
    #[serde(rename = "S_ForceLapsNb")]
    pub forced_nb_laps: i32,

    /// The number of rounds per warmup.
    #[serde(rename = "S_WarmUpNb")]
    pub nb_warmup_rounds: i32,

    /// The duration of one warmup round in seconds.
    #[serde(rename = "S_WarmUpDuration")]
    pub warmup_duration_secs: i32,

    /// Time limit before going to the next map in seconds.
    #[serde(rename = "S_TimeLimit")]
    pub time_limit_secs: i32,
}

/// Setting for the Champion game mode.
///
/// References:
/// - Libs/Nadeo/ModeLibs/Common/ModeBase.Script.txt (2020-06-23)
/// - Libs/Nadeo/ModeLibs/Common/ModeMatchmaking.Script.txt (2020-06-09)
/// - Libs/Nadeo/TMxSM/Race/ModeBase.Script.txt (2020-08-25)
/// - Modes/TrackMania/TM_Champion_Online.Script.txt (2020-09-10)
/// - https://doc.maniaplanet.com/dedicated-server/references/settings-list-for-nadeo-gamemodes
#[derive(Serialize, Deserialize, Debug)]
pub struct ChampionOptions {
    /// Chat time at the end of a map or match in seconds.
    #[serde(rename = "S_ChatTime")]
    pub chat_time_secs: i32,

    /// Forced number of laps.
    ///
    /// Set to -1 to use laps from map validation.
    /// Set to 0 to use "independent" laps (default in TimeAttack).
    #[serde(rename = "S_ForceLapsNb")]
    pub forced_nb_laps: i32,

    /// The number of rounds per warmup.
    #[serde(rename = "S_WarmUpNb")]
    pub nb_warmup_rounds: i32,

    /// The duration of one warmup round in seconds.
    #[serde(rename = "S_WarmUpDuration")]
    pub warmup_duration_secs: i32,
    // TODO support ChampionOptions
    //  - S_PointsRepartition: "20,14,12,10,8,7,6,5,5,4,4,3,3,2,2,1"
    //  - S_PointsLimit: -1
    //  - S_RoundsLimit: 6
    //  - S_PauseBeforeRoundNb: 0
    //  - S_PauseDuration: 360
    //  - S_WinnersRatio: 0.5
    //  - S_ForceWinnersNb: 0
    //  - S_TimeOutPlayersNumber: 0
    //  - S_FinishTimeout: 5
    //  - S_TimeLimit: -1
    //  - S_DisableGiveUp: false
    //  - S_UseTieBreak: false
    //  - S_BestLapBonusPoints: 2
    //  - S_RoundsWithAPhaseChange: "3,5"
    //  - S_EarlyEndMatchCallback: true
    //  - S_EndRoundPreScoreUpdateDuration: 5
    //  - S_EndRoundPostScoreUpdateDuration: 5
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CupOptions {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KnockoutOptions {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LapsOptions {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RoundsOptions {
    // TODO
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TeamsOptions {
    // TODO
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

    /// Formatted display name.
    #[serde(rename = "NickName")]
    pub display_name: DisplayString,

    /// (see functions)
    #[serde(rename = "Flags")]
    pub flag_digit_mask: i32,

    /// (see functions)
    #[serde(rename = "SpectatorStatus")]
    pub spectator_digit_mask: i32,

    #[serde(deserialize_with = "deserialize_opt_team_id")]
    pub team_id: Option<TeamId>,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniasetteampoints
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum TeamId {
    Blue,
    Red,
}

impl<'de> Deserialize<'de> for TeamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let id: i32 = serde::de::Deserialize::deserialize(deserializer)?;

        Ok(match id {
            0 => TeamId::Blue,
            1 => TeamId::Red,
            _ => panic!("unexpected team id {}", id),
        })
    }
}

fn deserialize_opt_team_id<'de, D>(deserializer: D) -> Result<Option<TeamId>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let id = i32::deserialize(deserializer)?;
    Ok(match id {
        -1 => None,
        0 => Some(TeamId::Blue),
        1 => Some(TeamId::Red),
        _ => panic!("unexpected team id {}", id),
    })
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
    pub name: DisplayString,

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

    /// Total duration of the lap up to this checkpoint.
    #[serde(rename = "laptime")]
    pub lap_time_millis: i32,

    /// Checkpoint index; or the number of unique checkpoints crossed
    /// since the beginning of this run minus one.
    #[serde(rename = "checkpointinrace")]
    pub race_cp_index: i32,

    /// Lap checkpoint index; or the number of unique checkpoints crossed
    /// since the beginning of this lap minus one.
    #[serde(rename = "checkpointinlap")]
    pub lap_cp_index: i32,

    /// `True` if the player has crossed the finish line.
    #[serde(rename = "isendrace")]
    pub is_finish: bool,

    /// `True` if the player has crossed the multi-lap line.
    #[serde(rename = "isendlap")]
    pub is_lap_finish: bool,

    /// Speed of the player in km/h at the time of passing this checkpoint.
    /// This is negative if they are driving backwards!
    pub speed: f32,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniaeventrespawn
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct CheckpointRespawnEvent {
    /// The driving player's login.
    #[serde(rename = "login")]
    pub player_login: String,

    /// The total number of respawns in this run so far.
    #[serde(rename = "nbrespawns")]
    pub nb_respawns: i32,

    /// Checkpoint index; or the number of unique checkpoints crossed
    /// since the beginning of this run minus one.
    #[serde(rename = "checkpointinrace")]
    pub race_cp_index: i32,

    /// Lap checkpoint index; or the number of unique checkpoints crossed
    /// since the beginning of this lap minus one.
    #[serde(rename = "checkpointinlap")]
    pub lap_cp_index: i32,
}

/// The ranking of the current race.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniascores
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct Scores {
    /// Empty, or an ID that was used when explicitly triggering the callback.
    #[serde(rename = "responseid", deserialize_with = "deserialize_response_id")]
    pub(in crate) response_id: Option<String>,

    /// Race ranking sorted from best to worst.
    pub players: Vec<PlayerScore>,

    /// Race ranking sorted from best to worst.
    pub teams: Vec<TeamScore>,

    /// Current progress of the match.
    #[serde(deserialize_with = "deserialize_section")]
    pub section: ScoresSection,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ScoresSection {
    Other,
    PreEndRound,
    EndRound,
    EndMap,
    EndMatch,
}

fn deserialize_section<'de, D>(deserializer: D) -> Result<ScoresSection, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use ScoresSection::*;

    let s = String::deserialize(deserializer)?;
    Ok(match s.as_ref() {
        "" => Other,
        "PreEndRound" => PreEndRound,
        "EndRound" => EndRound,
        "EndMap" => EndMap,
        "EndMatch" => EndMatch,
        _ => panic!("unexpected scores section {}", s),
    })
}

/// A player's ranking in the current race.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniascores
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct PlayerScore {
    /// The player's login.
    pub login: String,

    /// The player's formatted display name.
    #[serde(rename = "name")]
    pub display_name: DisplayString,

    /// Rank of the player in the current race.
    #[serde(rename = "rank")]
    pub pos: i32,

    /// Best run time in milliseconds (or -1 if no completed run).
    #[serde(rename = "bestracetime")]
    pub best_time_millis: i32,

    /// Checkpoints times during the best run in milliseconds (or empty if no completed run).
    #[serde(rename = "bestracecheckpoints")]
    pub best_time_cp_millis: Vec<i32>,

    /// Best lap time in milliseconds (or -1 if no completed lap).
    #[serde(rename = "bestlaptime")]
    pub best_lap_time_millis: i32,

    /// Checkpoints times during the best lap in milliseconds (or empty if no completed lap).
    #[serde(rename = "bestlapcheckpoints")]
    pub best_lap_time_cp_millis: Vec<i32>,

    #[serde(rename = "roundpoints")]
    pub points_round: i32,

    #[serde(rename = "mappoints")]
    pub points_map: i32,

    #[serde(rename = "matchpoints")]
    pub points_match: i32,
}

/// A team's ranking in the current race.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniascores
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct TeamScore {
    pub id: TeamId,

    pub name: DisplayString,

    #[serde(rename = "roundpoints")]
    pub points_round: i32,

    #[serde(rename = "mappoints")]
    pub points_map: i32,

    #[serde(rename = "matchpoints")]
    pub points_match: i32,
}

/// A string with in-game formatting.
#[derive(PartialEq, Eq, Clone)]
pub struct DisplayString {
    /// The formatted string.
    pub formatted: String,
}

impl DisplayString {
    pub fn from(str: String) -> Self {
        DisplayString { formatted: str }
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

impl std::fmt::Debug for DisplayString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.serialize_str(&self.plain())
    }
}

impl<'de> Deserialize<'de> for DisplayString {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let formatted: String = serde::de::Deserialize::deserialize(deserializer)?;
        Ok(DisplayString { formatted })
    }
}

impl Serialize for DisplayString {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.formatted)
    }
}

/// See `Callback::PlayerAnswered`.
#[derive(Debug, Clone)]
pub struct PlayerAnswer {
    /// The answer string, either from a Manialink (`<quad action="my_action"/>`),
    /// or from ManiaScript (`TriggerPageAction("my_action");`)
    pub answer: String,

    /// The current values of Manialink inputs like `<entry name="...">`
    /// or `<textedit name="...">`.
    pub entries: HashMap<String, String>,
}

/// Event data sent when starting or ending a warmup or pause.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#maniaplanetwarmupstatus
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct WarmupOrPauseStatus {
    /// Empty, or an ID that was used when explicitly triggering the callback.
    #[serde(rename = "responseid", deserialize_with = "deserialize_response_id")]
    pub(in crate) response_id: Option<String>,

    /// True if a warmup/pause is available in the game mode, false otherwise.
    pub available: bool,

    /// True if a warmup/pause is ongoing, false otherwise.
    pub active: bool,
}

/// Event data sent when a warmup round starts or ends.
///
/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniawarmupstartround
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct WarmupRoundStatus {
    /// The number of the current warmup round.
    #[serde(rename = "current")]
    pub current_round: i32,

    /// The total number of warmup rounds.
    #[serde(rename = "total")]
    pub nb_total_rounds: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(in crate) struct ManialinkEntry {
    pub name: std::string::String,
    pub value: std::string::String,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniaeventgiveup
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(in crate) struct GenericScriptEvent {
    pub login: std::string::String,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#maniaplanetstartserver_start
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(in crate) struct StartServerEvent {
    pub restarted: bool,
    pub mode: StartServerEventMode,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#maniaplanetstartserver_start
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(in crate) struct StartServerEventMode {
    pub updated: bool,
    pub name: String,
}

/// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#maniaplanetloadingmap_start
#[derive(Deserialize, Debug, PartialEq, Clone)]
pub(in crate) struct LoadingMapEvent {
    pub restarted: bool,
}

fn deserialize_response_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}

#[derive(Clone, Deserialize, Debug)]
pub struct ChampionScores {
    pub players: Vec<ChampionScore>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ChampionScore {
    #[serde(rename = "login")]
    pub player_login: String,

    pub round: ChampionScoreRank,

    pub step: ChampionScoreRank,
}

#[derive(Clone, Deserialize, Debug)]
pub struct ChampionScoreRank {
    pub rank: i32,
    pub points: i32,
}

#[derive(Clone, Deserialize, Debug)]
pub struct KnockoutEliminations {
    #[serde(rename = "accountids")]
    pub eliminated_account_ids: Vec<String>,
}
