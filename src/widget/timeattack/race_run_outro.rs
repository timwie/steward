use askama::Template;

/// A widget that summarizes a player's run.
///
/// # Sending
/// - Send this widget when the player finishes a run.
/// - Remove this widget when the player starts their next run.
#[derive(Template, Debug)]
#[template(path = "timeattack/race_run_outro.xml")]
pub struct RunOutroWidget {
    /// Compares this run to the personal best.
    /// If this is negative, this run has set a new personal best.
    /// If this is `None`, there was no personal best before.
    pub pb_diff_millis: Option<i32>,

    /// This player's map rank.
    /// Might have been set, improved, or unchanged with this run.
    pub record_pos: usize,

    /// The number of spots gained in the map ranking,
    /// or `0` if this run did not improve the player's map rank.
    pub record_pos_gained: usize,

    /// The player's new rank in this race.
    /// Might have been set, improved, or unchanged with this run.
    pub race_pos: usize,
}
