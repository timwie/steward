use serde::ser::{Error, SerializeStruct};
use serde::{Serialize, Serializer};

use crate::chat::{CommandOutputResponse, CommandResponse, DangerousCommand};

/// A widget that can be used for the outputs of chat commands.
/// Such outputs are not ideal in the chat, since you cannot
/// highlight & copy them.
#[derive(Serialize, Debug)]
pub struct PopupWidget<'a> {
    #[serde(flatten)]
    internal: PopupWidgetInternals<'a>,
}

impl PopupWidget<'_> {
    pub fn from(response: CommandResponse) -> PopupWidget {
        use CommandOutputResponse::*;
        use CommandResponse::*;

        let output = response.to_string();

        PopupWidget {
            internal: match response {
                Output(CurrentConfig { .. }) | Output(InvalidConfig { .. }) => {
                    PopupWidgetInternals::ConfigEditor(output)
                }
                Confirm(cmd, _) => PopupWidgetInternals::Confirm(output, cmd),
                _ => PopupWidgetInternals::Default(output),
            },
        }
    }
}

#[derive(Debug)]
enum PopupWidgetInternals<'a> {
    /// Use to display command outputs. Only action is 'close'.
    Default(String),

    /// Use for dangerous commands. Display a warning message, and
    /// offer to 'cancel', or 'confirm'.
    Confirm(String, DangerousCommand<'a>),

    /// Use only for the `/config` command. Display the config, and
    /// offer to 'cancel', or 'submit'.
    ConfigEditor(String),
}

impl Serialize for PopupWidgetInternals<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // TODO don't use vague numbers for the PopupWidget mode

        match self {
            PopupWidgetInternals::Default(output) => {
                let mut state = serializer.serialize_struct("", 2)?;
                state.serialize_field("mode", &0)?;
                state.serialize_field("output", &output)?;
                state.end()
            }
            PopupWidgetInternals::Confirm(output, cmd) => {
                let cmd = serde_json::to_string(cmd).map_err(S::Error::custom)?;

                let mut state = serializer.serialize_struct("", 3)?;
                state.serialize_field("mode", &1)?;
                state.serialize_field("output", &output)?;
                state.serialize_field("cmd", &cmd)?;
                state.end()
            }
            PopupWidgetInternals::ConfigEditor(output) => {
                let mut state = serializer.serialize_struct("", 2)?;
                state.serialize_field("output", &output)?;
                state.serialize_field("mode", &2)?;
                state.end()
            }
        }
    }
}
