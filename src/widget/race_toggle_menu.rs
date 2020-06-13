use std::cmp::Ordering;
use std::time::SystemTime;

use serde::{Serialize, Serializer};

use crate::controller::ActivePreferenceValue;
use crate::widget::Widget;

/// A widget displayed during the race, that can be toggled
/// by pressing a key. It contains
/// - the complete list of maps
///   - display a player's map rank for each map
///   - let the player change their map preferences
/// - the current server ranking
/// - the current map ranking
/// - the current race ranking
///
/// The data for the race ranking does not need to be included here,
/// as it's available via ManiaScript.
///
/// # Sending
/// - This widget has to be re-sent for each map, to load new map & server rankings.
///   The map list also has to be re-send, since we cannot update the map ranks and
///   queue priorities it displays.
/// - Within the same map, ManiaScript events can be used to update the race & map
///   ranking accordingly.
#[derive(Serialize, Debug)]
pub struct ToggleMenuWidget<'a> {
    pub map_list: MapList<'a>,

    pub map_ranking: MapRanking<'a>,

    pub server_ranking: ServerRanking<'a>,

    /// The maximum number of live scores displayed for the current race.
    pub max_displayed_race_ranks: usize,
}

impl Widget for ToggleMenuWidget<'_> {
    const FILE: &'static str = "race_toggle_menu.j2";
}

#[derive(Serialize, Debug)]
pub struct MapList<'a> {
    /// The server's playlist, sorted so that maps with worse or
    /// missing personal records are higher up.
    pub maps: Vec<MapListEntry<'a>>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct MapListEntry<'a> {
    /// UID of the map at this entry.
    pub map_uid: &'a str,

    /// Display name of the map at this entry.
    pub map_name: &'a str,

    /// Author of the map at this entry.
    pub map_author_login: &'a str,

    /// The player's preference for the map at this entry.
    pub preference: Option<ActivePreferenceValue>,

    /// The number of players that have completed a run
    /// on this map.
    pub nb_records: usize,

    /// The ranking of this player's personal best run
    /// within the ranking of records. Within `1..nb_records`.
    pub map_rank: Option<usize>,

    /// The moment this map was added to the server.
    #[serde(serialize_with = "format_duration_since")]
    pub added_since: SystemTime,

    /// `True` if this map is currently being played.
    /// This is significant, because the record stats will
    /// be out of date when new records are set. Consequently,
    /// the record stats should not be display when this is `True`.
    pub is_current_map: bool,

    /// The queue position of this map *at the start of the current race*.
    /// Since the priority changes whenever players change their preference,
    /// or admin force-queue maps, it cannot be up-to-date. Therefore, it is
    /// more of a suggestion. Large differences should be uncommon though.
    ///
    /// This is `0` if `is_current_map`.
    ///
    /// It can be read as "will be played in `<queue_pos>` maps".
    pub queue_pos: usize,
}

impl Ord for MapListEntry<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        // (a) Put current map at the top.
        if self.is_current_map {
            return Ordering::Less;
        }
        if other.is_current_map {
            return Ordering::Greater;
        }
        // (b) Sort worse map ranks higher up.
        //     None < Some(_), so having no record places them at the top.
        // (c) If map ranks are equal, put recently added maps first.
        let a = self.map_rank.map(|n| -(n as i32));
        let b = other.map_rank.map(|n| -(n as i32));
        a.cmp(&b)
            .then_with(|| self.added_since.cmp(&other.added_since))
    }
}

impl PartialOrd for MapListEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Serialize, Debug)]
pub struct MapRanking<'a> {
    /// A selection of top map ranks.
    pub entries: Vec<MapRankingEntry<'a>>,

    /// The player's own map rank, or `None` if they
    /// have not set a record on this map.
    pub personal_entry: Option<MapRankingEntry<'a>>,

    /// Limits the amount of entries.
    pub max_displayed_map_ranks: usize,
}

#[derive(Serialize, Debug)]
pub struct MapRankingEntry<'a> {
    /// The map rank.
    pub pos: usize,

    /// The player's nick name.
    pub nick_name: &'a str,

    /// The player's personal best.
    pub millis: usize,

    /// The moment this record was set.
    #[serde(serialize_with = "format_duration_since")]
    pub timestamp: SystemTime,

    /// `True` if this is the player's own record.
    pub is_own: bool,
}

#[derive(Serialize, Debug)]
pub struct ServerRanking<'a> {
    /// A selection of top server ranks.
    pub entries: Vec<ServerRankingEntry<'a>>,

    /// The player's own server rank, or `None` if
    /// they are unranked.
    pub personal_entry: Option<ServerRankingEntry<'a>>,

    /// Limits the amount of entries.
    pub max_displayed_server_ranks: usize,

    /// The maximum server rank; or the number of players that have a server rank.
    pub max_pos: usize,
}

#[derive(Serialize, Debug)]
pub struct ServerRankingEntry<'a> {
    /// The server rank.
    pub pos: usize,

    /// Formatted nick name of the player at this server rank.
    pub nick_name: &'a str,

    /// The number of beaten records, summed up for every map.
    pub nb_wins: usize,

    /// The number of records better than this player's, summed up for every map.
    pub nb_losses: usize,

    /// `True` if this is the player's own rank.
    pub is_own: bool,
}

/// Serialize the duration since the given timestamp as a readable string.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn format_duration_since<S>(x: &SystemTime, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let seconds_since = match x.elapsed() {
        Ok(duration) => duration.as_secs(),
        Err(_) => return s.serialize_str(""),
    };
    let days_since = seconds_since / 60 / 60 / 24; // div rounds down
    let weeks_since = days_since / 7;
    let months_since = days_since / 30;

    if days_since < 2 {
        return s.serialize_str("New");
    }
    if weeks_since < 2 {
        return s.serialize_str(&format!("{} days ago", days_since)); // "2..13 days ago"
    }
    if months_since < 2 {
        return s.serialize_str(&format!("{} weeks ago", weeks_since)); // "2..8 weeks ago"
    }
    if months_since >= 12 {
        return s.serialize_str("Long ago");
    }
    s.serialize_str(&format!("{} months ago", months_since)) // "2..11 months ago"
}
