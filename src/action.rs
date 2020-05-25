use serde::Deserialize;

use crate::database::PreferenceValue;

/// Actions are triggered within widgets via ManiaScript
/// and allow players to interact with the controller.
///
/// Each of the variants can be parsed from JSON objects, f.e.:
/// `{ "action": "VoteRestart", "vote": true }`
#[derive(Deserialize, Debug)]
#[serde(tag = "action")]
pub enum Action<'a> {
    /// Update a player's map preference.
    SetPreference {
        map_uid: &'a str,
        preference: PreferenceValue, // 1..3 in JSON
    },

    /// Update whether a player is for or against a restart
    /// of the current map.
    VoteRestart { vote: bool },
}

impl Action<'_> {
    /// Parse an action.
    ///
    /// # Panics
    /// Panics if the given string is not a valid JSON representation of any action.
    pub fn from_json(json_str: &str) -> Action {
        serde_json::from_str(&json_str).expect("failed to deserialize action")
    }
}
