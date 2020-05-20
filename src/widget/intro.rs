use std::collections::HashMap;

use serde::Serialize;

use crate::controller::ActivePreferenceValue;
use crate::database::PreferenceValue;
use crate::widget::Widget;

/// An introductory widget that is displayed when a map
/// is loaded, and hidden once the race starts.
///
/// The game will hide widgets during the MediaTracker intro,
/// but overall it's still long enough for players to see,
/// since there are short delays before that intro, and before
/// the start of the race.
///
/// # Sending
/// - Send this widget when a map starts
/// - Remove it once the player joins the race.
#[derive(Serialize, Debug)]
pub struct IntroWidget<'a> {
    /// The formatted map name.
    pub map_name: &'a str,

    /// The map author's login.
    pub map_author_login: &'a str,

    /// The map author's nick name, which can only be added & updated
    /// whenever the author joins the server.
    pub map_author_nick_name: Option<&'a str>,

    /// The player's map ranking, or `None` if they have not
    /// set any record.
    pub player_map_rank: Option<usize>,

    /// The number of players that have completed a run
    /// on this map.
    pub max_map_rank: usize,

    /// The player's preference for this map, if any.
    pub player_preference: Option<ActivePreferenceValue>,

    /// Counts the preferences of any player for this map,
    /// connected or not.
    pub preference_counts: HashMap<PreferenceValue, usize>,
}

impl Widget for IntroWidget<'_> {
    const FILE: &'static str = "intro.j2";
}
