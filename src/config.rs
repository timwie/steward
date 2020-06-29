use std::path::PathBuf;

use lazy_static::*;
use semver::Version;
use serde::{Deserialize, Serialize};

lazy_static! {
    /// Controller version.
    pub static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION")).expect("failed to parse our own SemVer");
}

/// User-Agent header for outgoing requests.
pub const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    ", (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Images used by widgets are located in the `src/res/img`, but need
/// to be hosted somewhere. Using jsDelivr, we can serve files from the GitHub
/// repository via their CDN.
///
/// Reference: https://www.jsdelivr.com/features#gh
// use @<branch>, @<tag>, or @latest (most recent tag)
pub const CDN_PREFIX: &str = concat!(
    "https://cdn.jsdelivr.net/gh/timwie/steward@v",
    env!("CARGO_PKG_VERSION"),
    "/src/res/img"
);

/// Same as `CDN_PREFIX`, but using images from the master branch.
/// Useful for development, but not for production, since images might disappear
/// for older versions.
pub const CDN_PREFIX_MASTER: &str = "https://cdn.jsdelivr.net/gh/timwie/steward@master/src/res/img";

/// The number of top records for which ghost replays are stored.
pub const MAX_GHOST_REPLAY_RANK: usize = 3;

/// The time (in percentage of the total outro duration) during which players
/// can still vote for a restart after the race ends. The next map will be
/// decided after this duration.
///
/// This should be long enough to let players interact with the poll, but also short
/// enough to be able to display the next map for a good duration within the outro.
pub const VOTE_DURATION_RATIO: f32 = 0.66;

/// Require this percentage of players for the first restart vote.
pub const DEFAULT_MIN_RESTART_VOTE_RATIO: f32 = 0.5;

/// Add this percentage to the restart vote threshold for subsequent restarts.
pub const MIN_RESTART_VOTE_RATIO_STEP: f32 = 0.25;

/// Limits the amount of top server ranks displayed.
///
/// This should be as low as necessary to display it in a widget
/// with limited vertical space.
pub const MAX_DISPLAYED_SERVER_RANKS: usize = 10;

/// Limits the amount of top map ranks displayed.
///
/// This should be as low as necessary to display it in a widget
/// with limited vertical space.
pub const MAX_DISPLAYED_MAP_RANKS: usize = 10;

/// Limits the amount of top race ranks displayed.
///
/// This should be as low as necessary to display it in a widget
/// with limited vertical space.
pub const MAX_DISPLAYED_RACE_RANKS: usize = 10;

/// Limits the amount of next maps in the queue displayed
/// during the outro.
///
/// This should be as low as necessary to display it in a widget
/// with limited vertical space.
pub const MAX_DISPLAYED_IN_QUEUE: usize = 5;

/// The maximum map record to announce to other players in chat when set.
///
/// Setting this too high might pollute the chat.
pub const MAX_ANNOUNCED_RECORD: usize = 10;

/// The maximum map record to announce to other players in chat when improved,
/// that is, new records that did improved the time, but not a player's rank.
/// For example, a new top 1 record should always be announced.
///
/// Setting this too high might pollute the chat.
pub const MAX_ANNOUNCED_RECORD_IMPROVEMENT: usize = 3;

/// The maximum server rank to announce to other players in chat when reached.
///
/// Setting this too high might pollute the chat.
pub const MAX_ANNOUNCED_RANK: usize = 10;

/// The maximum number of top server ranks announced to other players at once.
///
/// Setting this too high might pollute the chat.
pub const MAX_NB_ANNOUNCED_RANKS: usize = 3;

/// The milliseconds into a player's run after which temporary widgets
/// are hidden.
///
/// The idea is that we maximize the time you can display widgets in-between runs,
/// but still keep it low enough so that it is not a distraction when starting the next.
pub const START_HIDE_WIDGET_DELAY_MILLIS: u64 = 1500;

/// The file that will contain the list of blacklisted players.
pub const BLACKLIST_FILE: &str = "blacklist.txt";

/// Controller config.
#[derive(Deserialize, Serialize)]
pub struct Config {
    /// The address of the game server's XML-RPC port, f.e. "127.0.0.1:5000".
    ///
    /// A game server will listen on the port 5000 by default, where each
    /// additional instance will use 5001, 5002, etc. A game client
    /// will also reserve a port, which is relevant for development:
    /// if you start the game first, the server will listen at port 5001.
    ///
    /// It is also possible to select a specific port, using the `<xmlrpc_port>`
    /// setting in the server config.
    pub rpc_address: String,

    /// The "SuperAdmin" login defined in the `<authorization_levels>`
    /// server config in `/UserData/Config/*.txt`.
    pub rpc_login: String,

    /// The "SuperAdmin" password defined in the `<authorization_levels>`
    /// server config in `/UserData/Config/*.txt`.
    pub rpc_password: String,

    /// Connection configuration parsed from libpq-style connection strings, f.e.
    /// `host=127.0.0.1 port=5432 user=postgres password=123 connect_timeout=10`.
    ///
    /// Reference: https://www.postgresql.org/docs/9.3/libpq-connect.html#LIBPQ-CONNSTRING
    pub postgres_connection: String,

    /// List of player logins that can execute super admin commands.
    pub super_admin_whitelist: Vec<String>,

    /// List of player logins that can execute admin commands.
    pub admin_whitelist: Vec<String>,

    /// To calculate the time limit of a map, this factor is applied to either the
    /// author time or the top record.
    pub time_limit_factor: u32,

    /// The maximum time limit in seconds.
    pub time_limit_max_secs: u32,

    /// The minimum time limit in seconds.
    pub time_limit_min_secs: u32,

    /// The time spent on a map after the race ends in seconds.
    /// Overrides the `S_ChatTime` mode setting.
    ///
    /// This should be long enough to allow for widget interaction
    /// after a race.
    ///
    /// Votes during the outro will be open for two thirds of this value.
    pub outro_duration_secs: u32,
}

impl Config {
    /// The time during which players can still vote for a restart
    /// after a race ends. The next map will be decided after
    /// this duration.
    pub fn vote_duration_secs(&self) -> u32 {
        (self.outro_duration_secs as f32 * VOTE_DURATION_RATIO) as u32
    }

    /// Read the config file listed in the `STEWARD_CONFIG` environment variable.
    ///
    /// # Panics
    /// - when `STEWARD_CONFIG` is not set
    /// - when `STEWARD_CONFIG` does not point to a valid TOML config
    /// - when the file cannot be parsed
    pub fn load() -> Config {
        let f = Self::path().unwrap_or_else(|| {
            panic!("cannot locate config: use the '{}' env var", CONFIG_ENV_VAR)
        });
        let f_str = std::fs::read_to_string(f).expect("failed to read config file");
        let cfg: Config = toml::from_str(&f_str).expect("failed to parse config file");
        cfg
    }

    /// Overwrite the config file listed in the `STEWARD_CONFIG` environment variable.
    ///
    /// # Panics
    /// - when `STEWARD_CONFIG` is not set
    /// - when the file cannot be overwritten
    pub fn save(&self) {
        let mut config_str = toml::to_string(&self).expect("failed to compose config file");

        // Since all comments are removed from a previous config file,
        // we can at least add a link to the default config.
        let reference_link =
            "# Reference: https://github.com/timwie/steward/blob/master/config/steward.toml\n";
        config_str.insert_str(0, reference_link);

        let f = Self::path().unwrap_or_else(|| {
            panic!("cannot locate config: use the '{}' env var", CONFIG_ENV_VAR)
        });
        std::fs::write(f, config_str).expect("failed to overwrite config file");
    }

    fn path() -> Option<PathBuf> {
        match std::env::var(CONFIG_ENV_VAR) {
            Ok(f) => Some(PathBuf::from(f)).filter(|p| p.is_file()),
            Err(_) => None,
        }
    }
}

const CONFIG_ENV_VAR: &str = "STEWARD_CONFIG";
