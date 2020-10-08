use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::chat::DangerousCommand;
use crate::server::PlayerManialinkEvent;

/// Actions are triggered within widgets via ManiaScript
/// and allow players to interact with the controller.
///
/// Each of the variants can be parsed from JSON objects, f.e.:
/// `{ "action": "VoteRestart", "vote": true }`
///
/// # Limitations
/// ManiaScript's `TriggerPageAction` only supports single-line strings up to 128
/// characters. Trying to trigger larger actions will fail silently.
/// A workaround for this is to use `<entry name="...">` or `<textedit name="...">`.
/// Their contents can have any length, and will be included in the answer entries
/// when calling `TriggerPageAction`. See `InputAction` for such actions.
#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Action<'a> {
    /// Update a player's map preference.
    SetPreference {
        map_uid: &'a str,
        preference: ActivePreferenceValue, // 1..3 in JSON
    },

    /// Update whether a player is for or against a restart
    /// of the current map.
    VoteRestart { vote: bool },

    /// Confirm the execution of a pending, dangerous command.
    ConfirmCommand { cmd: DangerousCommand<'a> },

    /// Update the config, which is textually represented here.
    ///
    /// For this, we use a single text entry in a widget, so a config will
    /// have some format, and parsed from `repr`.
    #[serde(skip_serializing)]
    SetConfig { toml_config: String },
}

/// "Private" actions that are converted to an `Action` variant that
/// is built from the contents of `<entry name="...">` or `<textedit name="...">`
/// elements.
#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
enum InputAction {
    SetConfig { entry_name: String },
}

impl Action<'_> {
    /// Parse an action from a widget answer.
    ///
    /// # Panics
    /// Panics if `answer` is not a valid JSON representation of any action,
    /// or if there are missing entries in `entries`.
    pub fn from_answer(answer: &mut PlayerManialinkEvent) -> Action {
        if let Ok(action) = serde_json::from_str::<Action>(&answer.answer) {
            return action;
        }

        let action: InputAction =
            serde_json::from_str(&answer.answer).expect("failed to deserialize action");

        match action {
            InputAction::SetConfig {
                entry_name: entry_id,
            } => {
                let repr = answer
                    .entries
                    .remove(&entry_id)
                    .expect("missing config_input");

                Action::SetConfig { toml_config: repr }
            }
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum ActivePreferenceValue {
    None = 0,
    Pick = 1,
    Veto = 2,
    Remove = 3,
    #[serde(skip_deserializing)]
    AutoPick = 100,
}
