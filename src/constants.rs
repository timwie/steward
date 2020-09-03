use lazy_static::*;
use semver::Version;

lazy_static! {
    /// Controller version.
    pub static ref VERSION: Version = Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("failed to parse our own SemVer");
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
/// The master branch is used during development, while the files at the specific version tag
/// are used in production.
///
/// Reference: https://www.jsdelivr.com/features#gh
// use @<branch>, @<tag>, or @latest (most recent tag)
pub const fn cdn_prefix() -> &'static str {
    if cfg!(debug_assertions) {
        "https://cdn.jsdelivr.net/gh/timwie/steward@master/src/res/img"
    } else {
        concat!(
            "https://cdn.jsdelivr.net/gh/timwie/steward@v",
            env!("CARGO_PKG_VERSION"),
            "/src/res/img"
        )
    }
}

/// The file that will contain the list of blacklisted players.
pub const BLACKLIST_FILE: &str = "blacklist.txt";

pub const CONFIG_ENV_VAR: &str = "STEWARD_CONFIG";

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
pub const START_HIDE_WIDGET_DELAY_MILLIS: i64 = 1500;
