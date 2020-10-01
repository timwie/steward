use chrono::NaiveDateTime;
use serde::Serialize;

use crate::server::DisplayString;
use crate::widget::formatters::{format_narrow, format_record_age};

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

    /// The player's formatted display name.
    #[serde(serialize_with = "format_narrow")]
    pub display_name: &'a DisplayString,

    /// The player's personal best.
    pub millis: usize,

    /// The moment this record was set.
    #[serde(serialize_with = "format_record_age")]
    pub timestamp: NaiveDateTime,

    /// `True` if this is the player's own record.
    pub is_own: bool,
}
