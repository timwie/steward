use serde::Serialize;

use crate::widget::Widget;

/// A widget that compares sector times of both a player's
/// current run and personal best against the overall best record.
///
/// # Sending:
/// - Send this widget when a map starts.
/// - Within the same map, this widget can update itself, since the sector
///   times of any new run are available via ManiaScript.
#[derive(Serialize, Debug)]
pub struct SectorDiffWidget {
    /// The time of the player's personal best run on this map.
    pub pb_millis: usize,

    /// The sector times of this player's personal best run on this map.
    pub pb_sector_millis: Vec<usize>,

    /// The time of the overall best run on this map, set by any player.
    pub top1_millis: usize,

    /// The sector times of the overall best run on this map, set by any player.
    pub top1_sector_millis: Vec<usize>,
}

impl Widget for SectorDiffWidget {
    const FILE: &'static str = "race_sector_diff.j2";
}
