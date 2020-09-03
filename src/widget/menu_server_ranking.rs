use serde::Serialize;

use gbx::GameString;

use crate::widget::formatters::format_narrow;

/// A widget that displays the top server ranks.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - This widget has to be re-sent, since we cannot update the rankings.
#[derive(Serialize, Debug)]
pub struct ServerRankingWidget<'a> {
    #[serde(flatten)]
    pub ranking: ServerRanking<'a>,
}

#[derive(Serialize, Debug)]
pub struct ServerRanking<'a> {
    /// A selection of top server ranks.
    pub entries: Vec<ServerRankingEntry<'a>>,

    /// The player's own server rank, or `None` if
    /// they are unranked.
    pub personal_entry: Option<ServerRankingEntry<'a>>,

    /// The maximum server rank; or the number of players that have a server rank.
    pub max_pos: usize,
}

#[derive(Serialize, Debug)]
pub struct ServerRankingEntry<'a> {
    /// The server rank.
    pub pos: usize,

    /// Formatted nick name of the player at this server rank.
    #[serde(serialize_with = "format_narrow")]
    pub nick_name: &'a GameString,

    /// The number of beaten records, summed up for every map.
    pub nb_wins: usize,

    /// The number of records better than this player's, summed up for every map.
    pub nb_losses: usize,

    /// `True` if this is the player's own rank.
    pub is_own: bool,
}
