use serde::Serialize;

use crate::widget::formatters::format_queue_annotation;
use crate::widget::ActivePreferenceValue;

/// A widget displayed at the start of the outro, that lets
/// players update their preference, and vote for a map restart.
///
/// # Sending
/// - Send this widget when the outro starts.
/// - Remove it after a certain vote duration.
#[derive(Serialize, Debug)]
pub struct OutroQueueVoteWidget {
    /// The minimum percentage of players needed to vote
    /// in favour of a map restart.
    pub min_restart_vote_ratio: f32,

    /// The player's preference for the current map,
    /// at the end of the race.
    pub init_preference: Option<ActivePreferenceValue>,
}

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
}

#[derive(Serialize, Debug)]
pub struct OutroQueueEntry<'a> {
    /// The formatted map name.
    pub map_name: &'a str,

    #[serde(serialize_with = "format_queue_annotation")]
    pub annotation: QueueEntryAnnotation,
}

#[derive(Debug)]
pub enum QueueEntryAnnotation {
    None,
    Restart,
    Forced,
    PlayingNow,
}
