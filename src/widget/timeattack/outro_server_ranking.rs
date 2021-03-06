use askama::Template;

use crate::widget::timeattack::ServerRanking;

/// Summarize the changes to a player's server rank after a race,
/// and display a number of top ranks.
///
/// # Sending
/// - Send this widget after a race has ended, and the new ranking was calculated.
/// - Remove this widget when the next map starts.
#[derive(Template, Debug)]
#[template(path = "timeattack/outro_server_ranking.xml")]
pub struct OutroServerRankingWidget<'a> {
    /// The player's server rank.
    pub pos: usize,

    /// The number of players that are ranked.
    pub max_pos: usize,

    /// The difference of the player's record rank
    /// on this map, before & after the race.
    pub wins_gained: i32,

    /// The difference of the player's server rank,
    /// before & after the race.
    pub pos_gained: i32,

    /// A selection of top ranked players.
    pub server_ranking: ServerRanking<'a>,
}
