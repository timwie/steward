use askama::Template;

/// A widget that is displayed during a race to let players know their
/// - current server rank
/// - personal best on this map
/// - current map rank
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - Remove it when the race ends.
/// - This widget must be sent again whenever the player's record ranking changes,
///   whether that is because that player improved their time, or because another
///   player passed them in the map ranking.
/// - The server rank is static, since it will only be calculated in between maps
///   anyway.
/// - If the player improves their personal best without improving their record
///   rank, it must be updated in its ManiaScript.
#[derive(Template, Debug)]
#[template(path = "timeattack/race_live_ranks.xml")]
pub struct LiveRanksWidget {
    /// The player's personal best on this map, or `None`
    /// if they never completed a run on this map.
    pub pb_millis: Option<usize>,

    /// The map's top record set by any player, or `None`
    /// if no player completed a run on this map.
    pub top1_millis: Option<usize>,

    /// The rank of the player's personal best among all
    /// records on this map, or `None` if they have not
    /// completed a run on this map.
    pub map_rank: Option<usize>,

    /// The number of players that have set a record on this map, if any.
    pub max_map_rank: Option<usize>,

    /// The player's current server rank, or `None` if they are not ranked yet.
    pub server_rank: Option<usize>,

    /// The number of players that have a server rank.
    pub max_server_rank: Option<usize>,
}
