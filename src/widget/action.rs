use serde::Deserialize;

use crate::database::PreferenceValue;
use crate::server::PlayerAnswer;

/// Actions are triggered within widgets via ManiaScript
/// and allow players to interact with the controller.
///
/// Each of the variants can be parsed from JSON objects, f.e.:
/// `{ "action": "VoteRestart", "vote": true }`
///
/// # Limitations
/// ManiaScript's `TriggerPageAction` only supports strings up to 128
/// characters. Trying to trigger larger actions will fail silently.
/// A workaround for this is to use `<entry name="...">` or `<textedit name="...">`.
/// Their contents can have any length, and will be included in the answer entries
/// when calling `TriggerPageAction`.
#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Action {
    // no  &'a str, see https://github.com/serde-rs/serde/issues/1413#issuecomment-494892266
    /// Update a player's map preference.
    SetPreference {
        map_uid: String,
        preference: PreferenceValue, // 1..3 in JSON
    },

    /// Update whether a player is for or against a restart
    /// of the current map.
    VoteRestart { vote: bool },

    /// Confirm the execution of a pending, dangerous command.
    CommandConfirm,

    /// Cancel the execution of a pending, dangerous command.
    CommandCancel,

    /// Update the config, which is textually represented here.
    ///
    /// For this, we use a single text entry in a widget, so a config will
    /// have some format, and parsed from `repr`.
    SetConfig {
        #[serde(default)] // too long to include in the JSON string; use <textedit> entry
        repr: String,
    },
}

impl Action {
    /// Parse an action from a widget answer.
    ///
    /// # Panics
    /// Panics if `answer` is not a valid JSON representation of any action,
    /// or if there are missing entries in `entries`.
    pub fn from_answer(mut answer: PlayerAnswer) -> Action {
        let mut action: Action =
            serde_json::from_str(&answer.answer).expect("failed to deserialize action");

        if let Action::SetConfig { repr } = &mut action {
            // Read config string from Manialink entry:
            *repr = answer
                .entries
                .remove("config_input")
                .expect("missing config_input");
        }

        action
    }
}
