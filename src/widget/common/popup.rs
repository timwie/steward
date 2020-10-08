use askama::Template;

use crate::chat::{CommandOutputResponse, CommandResponse, DangerousCommand};
use crate::widget::filters;

/// A widget that can be used for the outputs of chat commands.
/// Such outputs are not ideal in the chat, since you cannot
/// highlight & copy them.
#[derive(Template, Debug)]
#[template(path = "common/popup.xml")]
pub struct PopupWidget<'a> {
    output: String,
    variant: PopupVariant<'a>,
}

impl PopupWidget<'_> {
    pub fn from(response: CommandResponse) -> PopupWidget {
        use CommandOutputResponse::*;
        use CommandResponse::*;

        let output = response.to_string();
        PopupWidget {
            output,
            variant: match response {
                Output(CurrentConfig { .. }) | Output(InvalidConfig { .. }) => {
                    PopupVariant::ConfigEditor
                }
                Confirm(cmd, _) => PopupVariant::Confirm { cmd },
                _ => PopupVariant::Default,
            },
        }
    }
}

#[derive(Debug)]
enum PopupVariant<'a> {
    /// Use to display command outputs. Only action is 'close'.
    Default,

    /// Use for dangerous commands. Display a warning message, and
    /// offer to 'cancel', or 'confirm'.
    Confirm { cmd: DangerousCommand<'a> },

    /// Use only for the `/config` command. Display the config, and
    /// offer to 'cancel', or 'submit'.
    ConfigEditor,
}
