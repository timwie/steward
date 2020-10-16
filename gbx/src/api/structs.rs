use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Dedicated server version information.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ServerBuildInfo {
    /// This should be "Trackmania".
    pub name: String,

    /// The build's version number, f.e. "3.3.0".
    pub version: String,

    /// The build's version date, f.e. "2020-07-01_14_30".
    #[serde(rename = "Build")]
    pub version_date: String,
}

/// Dedicated server options.
///
/// These options default to the values of the `<dedicated>` config in `.../UserData/Config/*.txt`
///
/// Any `next_*` option will become active as the `current_*` option on map change.
#[derive(Serialize, Deserialize, Debug)]
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
    /// (see http://www.tm-forum.com/viewtopic.php?p=130016#p130016)
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
    ///
    /// This should always be set to 0.
    /// (see http://www.tm-forum.com/viewtopic.php?p=14486#p14486)
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

    /// Players with a higher latency than this value will experience difficulties playing,
    /// but a lower value will reduce the CPU usage for players.
    ///
    /// Default: 200ms; Maximum: 540ms
    ///
    /// This value should be at least half of the highest ping that you would like to have.
    /// (see https://forums.ubisoft.com/showthread.php/2242192?p=15049698#post15049698)
    ///
    /// Config: `<clientinputs_maxlatency>` in `<server_options>`
    pub client_inputs_max_latency: i32,
}

/// Dedicated server network stats.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ServerNetStats {
    /// This value might be useful to check that a server has not been online
    /// for more than 30 days. Apparently this can prevent players from joining the server.
    /// (see https://doc.maniaplanet.com/dedicated-server/frequent-errors)
    #[serde(rename = "Uptime")]
    pub uptime_secs: i32,
}

/// Game mode information.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ModeInfo {
    /// The script that implements this mode.
    #[serde(rename = "Name")]
    pub script: ModeScript,

    /// All compatible map types for this mode.
    #[serde(deserialize_with = "deserialize_map_types")]
    pub compatible_map_types: HashSet<MapType>,

    /// Development: The version date of the mode script, f.e. "2020-09-10".
    /// A change in version might be of note.
    #[serde(rename = "Version")]
    pub version_date: String,
}

/// The type a map was validated for.
///
/// Map types are scripts that set certain requirements for a map.
/// The default `Race` type f.e. requires exactly one start block and at least one finish block.
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum ModeScript {
    Champion,
    Cup,
    Knockout,
    Laps,
    Rounds,
    Teams,
    TimeAttack,
    Custom {
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
            Custom { file_name } => file_name,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ModeScript::Custom { file_name } => file_name
                .trim_start_matches("Trackmania/")
                .trim_end_matches(".Script.txt"),
            _ => self
                .file_name()
                .trim_start_matches("Trackmania/TM_")
                .trim_end_matches("_Online.Script.txt"),
        }
    }

    /// The game's default game modes.
    ///
    /// These game modes that come with the dedicated server, and do not have to be
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
            None => ModeScript::Custom { file_name },
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

/// Settings for the TimeAttack game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct TimeAttackOptions {
    /// Chat time at the end of a map in seconds.
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

/// Settings for the Champion game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct ChampionOptions {
    // TODO ChampionOptions
}

/// Settings for the Cup game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct CupOptions {
    // TODO CupOptions
}

/// Settings for the Knockout game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct KnockoutOptions {
    // TODO KnockoutOptions
}

/// Settings for the Laps game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct LapsOptions {
    // TODO LapsOptions
}

/// Settings for the Rounds game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct RoundsOptions {
    // TODO RoundsOptions
}

/// Settings for the Teams game mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct TeamsOptions {
    // TODO TeamsOptions
}

/// Information for a connected player.
#[derive(Deserialize, Clone)]
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

    /// The current team of this player, if any.
    #[serde(deserialize_with = "deserialize_opt_team_id")]
    pub team_id: Option<TeamId>,

    /// (see functions)
    #[serde(rename = "Flags")]
    pub flag_digit_mask: i32,

    /// (see functions)
    #[serde(rename = "SpectatorStatus")]
    pub spectator_digit_mask: i32,
}

/// An identifier for teams in team-based game modes.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
#[derive(Debug, PartialEq, Eq)]
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

/// A map that is currently in the server's playlist.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistMap {
    /// A unique identifier.
    #[serde(rename = "UId")]
    pub uid: String,

    /// The map's file name in `.../UserData/Maps`.
    pub file_name: String,
}

impl PlaylistMap {
    /// `True` if this map is a Nadeo campaign map.
    ///
    /// The server's playlist can include bundled maps, that are not stored in `.../UserData/Maps`,
    /// with a file name like this: `Campaigns/CurrentQuarterly/Summer 2020 - 01.Map.Gbx`
    pub fn is_campaign_map(&self) -> bool {
        self.file_name.starts_with("Campaigns/")
    }
}

/// Information of a map in `.../UserData/Maps`.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MapInfo {
    /// A unique identifier.
    #[serde(rename = "UId")]
    pub uid: String,

    /// The formatted map name.
    pub name: DisplayString,

    /// The map's file path relative to `.../UserData/Maps`.
    pub file_name: String,

    /// The map author's login.
    #[serde(rename = "Author")]
    pub author_login: String,

    /// The "author time" in milliseconds. This is the time the map
    /// was validated with in the map editor.
    #[serde(rename = "AuthorTime")]
    pub author_millis: i32,
}

/// Event data produced when players cross a checkpoint or finish line.
#[derive(Deserialize, Debug, Clone)]
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

    /// The total durations of this run at the time of passing each checkpoint.
    ///
    /// This array is not filled by default - to change the behavior, call the
    /// script method `Trackmania.Event.SetCurRaceCheckpointsMode`:
    ///  - `always` will fill the array at each waypoint
    ///  - `never` (default) will never fill the array
    ///  - `endlap` will fill the array only if the player finished a lap
    ///  - `endrace` will fill the array only if the player finished the race
    #[serde(rename = "curracecheckpoints")]
    pub race_cp_millis: Vec<i32>,

    /// The total durations of this lap at the time of passing each checkpoint.
    ///
    /// This array is not filled by default - to change the behavior, call the
    /// script method `Trackmania.Event.SetCurLapCheckpointsMode`:
    ///  - `always` will fill the array at each waypoint
    ///  - `never` (default) will never fill the array
    ///  - `endlap` will fill the array only if the player finished a lap
    ///  - `endrace` will fill the array only if the player finished the race
    #[serde(rename = "curlapcheckpoints")]
    pub lap_cp_millis: Vec<i32>,

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

/// Event data produced when a player respawns at the previous checkpoint.
#[derive(Deserialize, Debug, Clone)]
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

/// Scores of the current match.
#[derive(Deserialize, Debug, Clone)]
pub struct Scores {
    #[serde(rename = "responseid", deserialize_with = "deserialize_response_id")]
    pub(in crate) response_id: Option<String>,

    /// Race ranking sorted from best to worst.
    pub players: Vec<PlayerScore>,

    /// Race ranking sorted from best to worst.
    pub teams: Vec<TeamScore>,

    /// The mode script section at which this event was triggered.
    #[serde(deserialize_with = "deserialize_section")]
    pub(in crate) section: Option<ScoresSection>,
}

/// Mode script sections that can trigger the `Trackmania.Scores` callback.
#[derive(Debug, Clone)]
pub enum ScoresSection {
    PreEndRound,
    EndRound,
    EndMap,
    EndMatch,
}

fn deserialize_section<'de, D>(deserializer: D) -> Result<Option<ScoresSection>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    use ScoresSection::*;

    let s = String::deserialize(deserializer)?;
    Ok(match s.as_ref() {
        "" => None,
        "PreEndRound" => Some(PreEndRound),
        "EndRound" => Some(EndRound),
        "EndMap" => Some(EndMap),
        "EndMatch" => Some(EndMatch),
        _ => panic!("unexpected scores section {}", s),
    })
}

/// A player's score in the current match.
#[derive(Deserialize, Debug, Clone)]
pub struct PlayerScore {
    /// The player's login.
    pub login: String,

    /// Yet another identifier for players.
    #[serde(rename = "accountid")]
    pub account_id: String,

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

    /// Points collected by this team.
    #[serde(flatten)]
    pub points: Points,
}

/// A team's score in the current match.
#[derive(Deserialize, Debug, Clone)]
pub struct TeamScore {
    pub id: TeamId,

    /// The team's formatted display name.
    pub name: DisplayString,

    /// Points collected by this team.
    #[serde(flatten)]
    pub points: Points,
}

/// Point scores of a player or team.
///
/// Different game modes will use different types of points:
///  - Champion: round, map, match (but there is only one round per map)
///  - Cup: round, map, match
///  - Knockout: match (players' final points are the number of total players minus their own rank)
///  - Laps: none
///  - Rounds: round, map, match
///  - Teams: round, map, match
///  - TimeAttack: none
#[derive(Deserialize, Debug, Clone)]
pub struct Points {
    /// Points collected in the current round.
    #[serde(rename = "roundpoints")]
    pub round: Option<i32>,

    /// Points collected on the current map.
    #[serde(rename = "mappoints")]
    pub map: Option<i32>,

    /// Points collected in the current match.
    #[serde(rename = "matchpoints")]
    pub match_: Option<i32>,
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
    /// (see https://wiki.xaseco.org/wiki/Text_formatting)
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// Event data produced when a player interacts with a Manialink.
#[derive(Debug, Clone)]
pub struct PlayerManialinkEvent {
    /// The answer string, either from a Manialink (`<quad action="my_action"/>`),
    /// or from ManiaScript (`TriggerPageAction("my_action");`)
    pub answer: String,

    /// The current values of Manialink inputs like `<entry name="...">`
    /// or `<textedit name="...">`.
    pub entries: HashMap<String, String>,
}

/// Current status of a warmup.
#[derive(Deserialize, Debug, Clone)]
pub struct WarmupStatus {
    #[serde(rename = "responseid", deserialize_with = "deserialize_response_id")]
    pub(in crate) response_id: Option<String>,

    /// True if a warmup is available in the game mode, false otherwise.
    ///
    /// Warmups are available for all default modes.
    pub available: bool,

    /// True if a warmup is ongoing, false otherwise.
    pub active: bool,
}

/// Current status of a pause.
#[derive(Deserialize, Debug, Clone)]
pub struct PauseStatus {
    #[serde(rename = "responseid", deserialize_with = "deserialize_response_id")]
    pub(in crate) response_id: Option<String>,

    /// True if a pause is available in the game mode, false otherwise.
    ///
    /// Pauses are available for all round-based default modes:
    ///  - Cup
    ///  - Champion
    ///  - Knockout
    ///  - Rounds
    ///  - Teams
    ///
    /// Pauses are *not* available for:
    ///  - Laps
    ///  - TimeAttack
    pub available: bool,

    /// True if a pause is ongoing, false otherwise.
    pub active: bool,
}

/// Event data produced when a warmup round starts or ends.
#[derive(Deserialize, Debug, Clone)]
pub struct WarmupRoundStatus {
    /// The number of the current warmup round.
    #[serde(rename = "current")]
    pub current_round: i32,

    /// The total number of warmup rounds.
    #[serde(rename = "total")]
    pub nb_total_rounds: i32,
}

/// Event data produced at the end of a Champion round.
#[derive(Clone, Deserialize, Debug)]
pub struct ChampionEndRoundEvent {
    #[serde(rename = "players")]
    pub scores: Vec<ChampionScoreOverall>,
}

/// Player score in the Champion mode.
#[derive(Clone, Deserialize, Debug)]
pub struct ChampionScoreOverall {
    #[serde(rename = "login")]
    pub player_login: String,

    /// Collected points and ranking in the current round.
    pub round: ChampionScore,

    /// Collected points and ranking in the current match.
    pub step: ChampionScore,
}

/// Player score in a step or round of the Champion mode.
#[derive(Clone, Deserialize, Debug)]
pub struct ChampionScore {
    pub rank: i32,
    pub points: i32,
}

/// Event data produced at the end of a Knockout round.
#[derive(Clone, Deserialize, Debug)]
pub struct KnockoutEndRoundEvent {
    /// The account IDs of players that were eliminated in this round.
    #[serde(rename = "accountids")]
    pub eliminated_account_ids: Vec<String>,
}

fn deserialize_response_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(if s.is_empty() { None } else { Some(s) })
}
