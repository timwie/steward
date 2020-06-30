use std::cmp::Ordering;

use chrono::NaiveDateTime;
use serde::Serialize;

use crate::controller::ActivePreferenceValue;
use crate::widget::ser::{format_last_played, format_map_age, format_record_age};
use crate::widget::Widget;

/// A widget displayed during the race, that can be toggled by pressing a key.
/// This widget is only responsible for displaying the menu frame - the actual
/// content is provided by other "sub-widgets". These are displayed on top
/// of the menu frame.
///
/// # Sending
/// - Send this widget to a player after the intro.
#[derive(Serialize, Debug)]
pub struct MenuWidget {}

impl Widget for MenuWidget {
    const FILE: &'static str = "menu.j2";
}

/// A widget that displays the server's playlist, and lets players change their map preferences.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - This widget has to be re-sent, since we cannot update the map ranks and
///   queue positions it displays.
#[derive(Serialize, Debug)]
pub struct PlaylistWidget<'a> {
    /// The server's playlist, sorted so that maps with worse or
    /// missing personal records are higher up. The first entry is
    /// the current map.
    pub entries: Vec<PlaylistWidgetEntry<'a>>,
}

impl Widget for PlaylistWidget<'_> {
    const FILE: &'static str = "menu_playlist.j2";
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct PlaylistWidgetEntry<'a> {
    /// UID of the map at this entry.
    pub map_uid: &'a str,

    /// Display name of the map at this entry.
    pub map_name: &'a str,

    /// Author of the map at this entry.
    pub map_author_login: &'a str,

    /// The player's preference for the map at this entry.
    pub preference: ActivePreferenceValue,

    /// The number of players that have completed a run
    /// on this map.
    pub nb_records: usize,

    /// The ranking of this player's personal best run
    /// within the ranking of records. Within `1..nb_records`.
    pub map_rank: Option<usize>,

    /// The moment this map was added to the server.
    #[serde(serialize_with = "format_map_age")]
    pub added_since: NaiveDateTime,

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

    /// The most recent time this player has played this map, or `None` if
    /// they have never played it. "Playing" means "finishing" here.
    #[serde(serialize_with = "format_last_played")]
    pub last_played: Option<NaiveDateTime>,
}

impl Ord for PlaylistWidgetEntry<'_> {
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

impl PartialOrd for PlaylistWidgetEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A widget that displays the top map records.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - Does not have to be re-sent when there are new records,
///   since they can be added in the script.
#[derive(Serialize, Debug)]
pub struct MapRankingWidget<'a> {
    #[serde(flatten)]
    pub ranking: MapRanking<'a>,
}

impl Widget for MapRankingWidget<'_> {
    const FILE: &'static str = "menu_map_ranking.j2";
}

#[derive(Serialize, Debug)]
pub struct MapRanking<'a> {
    /// A selection of top map ranks.
    pub entries: Vec<MapRankingEntry<'a>>,

    /// The player's own map rank, or `None` if they
    /// have not set a record on this map.
    pub personal_entry: Option<MapRankingEntry<'a>>,

    /// The maximum map rank; or the number of players that set a record on this map.
    pub max_pos: usize,
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
    #[serde(serialize_with = "format_record_age")]
    pub timestamp: NaiveDateTime,

    /// `True` if this is the player's own record.
    pub is_own: bool,
}

/// A widget that displays the top server ranks.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - This widget has to be re-sent, since we cannot update the rankings.
#[derive(Serialize, Debug)]
pub struct ServerRankingWidget<'a> {
    #[serde(flatten)]
    pub ranking: ServerRanking<'a>,
}

impl Widget for ServerRankingWidget<'_> {
    const FILE: &'static str = "menu_server_ranking.j2";
}

#[derive(Serialize, Debug)]
pub struct ServerRanking<'a> {
    /// A selection of top server ranks.
    pub entries: Vec<ServerRankingEntry<'a>>,

    /// The player's own server rank, or `None` if
    /// they are unranked.
    pub personal_entry: Option<ServerRankingEntry<'a>>,

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

/// A widget that displays the schedule, with the maps that are currently
/// at the top of the queue.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - Has to be re-sent whenever the top of the queue changes.
#[derive(Serialize, Debug)]
pub struct ScheduleWidget {
    // TODO add schedule widget details
//  - map name, author
//  - personal preferences
//  - minutes until played
}

impl Widget for ScheduleWidget {
    const FILE: &'static str = "menu_schedule.j2";
}
