use std::fmt::Display;

use serde::export::Formatter;

use crate::chat::message::{pluralize, write_start_message};

/// Chat messages from the controller to a specific player.
///
/// Note: messages should typically convey information that is
/// not already conveyed by widgets.
pub enum PlayerMessage {
    /// Remind a player to change their preferences to influence the queue.
    PreferenceReminder { nb_active_preferences: usize },
}

impl Display for PlayerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use PlayerMessage::*;

        write_start_message(f)?;

        match self {
            PreferenceReminder {
                nb_active_preferences: 0,
            } => {
                write!(f, "Don't like this map? ")?;
                write!(f, "Make sure to set your preferences in the map list.")
            }

            PreferenceReminder {
                nb_active_preferences: nb,
            } => {
                write!(
                    f,
                    "You are influencing the map queue with {}.",
                    pluralize("preference", *nb)
                )?;
                write!(
                    f,
                    "Make sure to change them to your liking by bringing up the map list."
                )
            }
        }
    }
}
