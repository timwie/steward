use serde::{Serialize, Serializer};

use crate::controller::{ActivePreferenceValue, QueuePriority};
use crate::widget::Widget;

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

    /// The priority can convey why a map will be queued.
    #[serde(serialize_with = "format_priority")]
    pub priority: QueuePriority,
}

fn format_priority<S>(p: &QueuePriority, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use QueuePriority::*;
    let str = match p {
        NoRestart => "Playing Now".to_string(),
        VoteRestart => "Restart".to_string(),
        Force(_) => "Force".to_string(),
        Score(score) if *score >= 0 => format!("+{}", *score),
        Score(score) => score.to_string(),
    };
    s.serialize_str(&str)
}

impl Widget for OutroQueueVoteWidget {
    const FILE: &'static str = "outro_queue_vote.j2";

    const ID: &'static str = "outro_poll";
}

impl Widget for OutroQueueWidget<'_> {
    const FILE: &'static str = "outro_queue.j2";

    const ID: &'static str = "outro_poll";
}
