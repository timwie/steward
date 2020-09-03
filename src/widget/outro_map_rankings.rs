use serde::Serialize;

use crate::widget::MapRanking;

/// A widget displayed during the outro, showing a ranking of
/// - top scores of the past race,
/// - and the updated list of map records.
///
/// The data for the race ranking does not need to be included here,
/// as it's available via ManiaScript.
///
/// # Sending
/// - Send this widget when the race ends.
/// - Remove this widget when the next map starts.
#[derive(Serialize, Debug)]
pub struct OutroMapRankingsWidget<'a> {
    /// A selection of the top records on this map. If the player's own
    /// record is not part of that list, add it to the end.
    pub map_ranking: MapRanking<'a>,

    /// The maximum number of live scores displayed for the current race.
    pub max_displayed_race_ranks: usize,
}
