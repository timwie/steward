use askama::Template;

use crate::widget::timeattack::MapRanking;
use crate::widget::ActivePreferenceValue;

#[derive(Template, Debug)]
#[template(path = "timeattack/outro.xml")]
pub struct OutroWidget<'a> {
    /// A selection of the top records on the map that was just played.
    /// If the player's own record is not part of that list, add it to the end.
    pub map_ranking: MapRanking<'a>,

    /// The maximum number of live scores displayed for the previous race.
    pub max_displayed_race_ranks: usize,

    /// The minimum percentage of players needed to vote in favour of a map restart.
    pub min_restart_vote_ratio: f32,

    /// The player's preference for the current map, at the end of the race.
    pub init_preference: Option<ActivePreferenceValue>,

    pub outro_duration_secs: u32,
    pub vote_duration_secs: u32,
}
