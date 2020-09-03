use chrono::NaiveDateTime;
use serde::Serialize;

use gbx::GameString;

use crate::database::PreferenceValue;
use crate::widget::formatters::format_queue_annotation;
use crate::widget::formatters::{format_last_played, format_narrow};
use crate::widget::ActivePreferenceValue;

/// A widget displayed during the outro and after the vote,
/// to let players know what the next maps are.
///
/// # Updates
/// - Send this widget after the vote ends.
/// - Remove it when the next map starts.
#[derive(Serialize, Debug)]
pub struct OutroQueueWidget<'a> {
    /// A number of maps with the highest priority,
    /// the first item being the map that is played next.
    pub next_maps: Vec<OutroQueueEntry<'a>>,

    /// `True` if enough players voted in favour of
    /// a restart of the current map.
    pub is_restart: bool,

    pub next_map: MapPreview<'a>,
}

#[derive(Serialize, Debug, Clone)]
pub struct OutroQueueEntry<'a> {
    /// The formatted map name.
    #[serde(serialize_with = "format_narrow")]
    pub map_name: &'a GameString,

    #[serde(serialize_with = "format_queue_annotation")]
    pub annotation: QueueEntryAnnotation,
}

#[derive(Debug, Clone)]
pub enum QueueEntryAnnotation {
    None,
    Restart,
    Forced,
    PlayingNow,
}

#[derive(Serialize, Debug)]
pub struct MapPreview<'a> {
    /// The formatted map name.
    #[serde(serialize_with = "format_narrow")]
    pub map_name: &'a GameString,

    /// The map author's nick name, which can only be added & updated
    /// whenever the author joins the server.
    #[serde(serialize_with = "format_narrow")]
    pub map_author_nick_name: &'a GameString,

    /// The player's map ranking, or `None` if they have not
    /// set any record.
    pub player_map_rank: Option<usize>,

    /// The number of players that have completed a run
    /// on this map.
    pub max_map_rank: usize,

    /// The player's preference for this map, if any.
    pub player_preference: ActivePreferenceValue,

    /// Counts the preferences of any player for this map,
    /// connected or not.
    pub preference_counts: Vec<(PreferenceValue, usize)>,

    /// The most recent time this player has played this map, or `None` if
    /// they have never played it. "Playing" means "finishing" here.
    #[serde(serialize_with = "format_last_played")]
    pub last_played: Option<NaiveDateTime>,
}
