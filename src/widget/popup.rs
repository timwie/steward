use serde::Serialize;
use serde_repr::Serialize_repr;

use crate::chat::{CommandOutputResponse, CommandResponse};

/// A widget that can be used for the outputs of chat commands.
/// Such outputs are not ideal in the chat, since you cannot
/// highlight & copy them.
#[derive(Serialize, Debug)]
pub struct PopupWidget<'a> {
    pub output: &'a str,
    pub mode: PopupMode,
}

#[derive(Serialize_repr, Debug)]
#[repr(u8)]
pub enum PopupMode {
    /// Use to display command outputs. Only action is 'close'.
    Default = 0,

    /// Use for dangerous commands. Display a warning message, and
    /// offer to 'cancel', or 'confirm'.
    Confirm = 1,

    /// Use only for the `/config` command. Display the config, and
    /// offer to 'cancel', or 'submit'.
    ConfigEditor = 2,
}

impl PopupMode {
    pub fn from(response: &CommandResponse<'_>) -> PopupMode {
        use CommandOutputResponse::*;
        use CommandResponse::*;

        match response {
            Output(CurrentConfig { .. }) | Output(InvalidConfig { .. }) => PopupMode::ConfigEditor,
            Confirm(_) => PopupMode::Confirm,
            _ => PopupMode::Default,
        }
    }
}
