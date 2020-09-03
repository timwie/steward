use serde::Serialize;

/// A widget displayed during the race, that can be toggled by pressing a key.
/// This widget is only responsible for displaying the menu frame - the actual
/// content is provided by other "sub-widgets". These are displayed on top
/// of the menu frame.
///
/// # Sending
/// - Send this widget to a player after the intro.
#[derive(Serialize, Debug)]
pub struct MenuWidget {}
