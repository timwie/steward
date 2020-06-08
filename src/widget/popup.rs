use serde::Serialize;
use serde_repr::Serialize_repr;

use crate::command::CommandResponse;
use crate::widget::Widget;

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
    /// Only action is close.
    Default = 0,

    /// Actions are close, or confirm.
    Confirm = 1,
}

impl PopupMode {
    pub fn from(response: &CommandResponse<'_>) -> PopupMode {
        match response {
            CommandResponse::Confirm(_) => PopupMode::Confirm,
            _ => PopupMode::Default,
        }
    }
}

impl Widget for PopupWidget<'_> {
    const FILE: &'static str = "popup.j2";
}
