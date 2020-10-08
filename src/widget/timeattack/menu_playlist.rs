use core::cmp::{Ord, Ordering, PartialOrd};
use core::option::Option::Some;

use askama::Template;
use chrono::NaiveDateTime;

use crate::server::DisplayString;
use crate::widget::filters;
use crate::widget::ActivePreferenceValue;

/// A widget that displays the server's playlist, and lets players change their map preferences.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - This widget has to be re-sent, since we cannot update the map ranks and
///   queue positions it displays.
#[derive(Template, Debug)]
#[template(path = "timeattack/menu_playlist.xml")]
pub struct PlaylistWidget<'a> {
    pub cdn: &'a str,

    /// The server's playlist, sorted so that maps with worse or
    /// missing personal records are higher up. The first entry is
    /// the current map.
    pub entries: Vec<PlaylistWidgetEntry<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlaylistWidgetEntry<'a> {
    /// UID of the map at this entry.
    pub map_uid: &'a str,

    /// Display name of the map at this entry.
    pub map_name: &'a DisplayString,

    /// Author of the map at this entry.
    pub map_author_display_name: &'a DisplayString,

    /// The player's preference for the map at this entry.
    pub preference: ActivePreferenceValue,

    /// The number of players that have completed a run or lap
    /// on this map.
    pub nb_records: usize,

    /// The ranking of this player's personal best run or lap
    /// within the ranking of records. Within `1..nb_records`.
    pub map_rank: Option<usize>,

    /// The moment this map was added to the server.
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
