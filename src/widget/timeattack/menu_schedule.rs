use askama::Template;

/// A widget that displays the schedule, with the maps that are currently
/// at the top of the queue.
///
/// # Sending
/// - Send this widget to a player after the intro.
/// - Has to be re-sent whenever the top of the queue changes.
#[derive(Template, Debug)]
#[template(path = "timeattack/menu_schedule.xml")]
pub struct ScheduleWidget {
    // TODO add schedule widget details
//  - map name, author
//  - personal preferences
//  - minutes until played
}
