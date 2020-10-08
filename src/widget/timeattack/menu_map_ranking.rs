use askama::Template;
use chrono::NaiveDateTime;

use crate::server::DisplayString;
use crate::widget::filters;

/// A widget that displays the top map records.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - Does not have to be re-sent when there are new records,
///   since they can be added in the script.
#[derive(Template, Debug)]
#[template(path = "timeattack/menu_map_ranking.xml")]
pub struct MapRankingWidget<'a> {
    pub ranking: MapRanking<'a>,
}

#[derive(Debug)]
pub struct MapRanking<'a> {
    /// A selection of top map ranks.
    pub entries: Vec<MapRankingEntry<'a>>,

    /// The player's own map rank, or `None` if they
    /// have not set a record on this map.
    pub personal_entry: Option<MapRankingEntry<'a>>,

    /// The maximum map rank; or the number of players that set a record on this map.
    pub max_pos: usize,
}

#[derive(Debug)]
pub struct MapRankingEntry<'a> {
    /// The map rank.
    pub pos: usize,

    /// The player's formatted display name.
    pub display_name: &'a DisplayString,

    /// The player's personal best.
    pub millis: usize,

    /// The moment this record was set.
    pub timestamp: NaiveDateTime,

    /// `True` if this is the player's own record.
    pub is_own: bool,
}
