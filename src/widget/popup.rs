use serde::Serialize;

use crate::widget::Widget;

/// A widget that can be used for the outputs of chat commands.
/// Such outputs are not ideal in the chat, since you cannot
/// highlight & copy them.
#[derive(Serialize, Debug)]
pub struct PopupWidget<'a> {
    pub output: &'a str,
}

impl Widget for PopupWidget<'_> {
    const FILE: &'static str = "popup.j2";
}
