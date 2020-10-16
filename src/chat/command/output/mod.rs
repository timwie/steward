use std::fmt::{Display, Formatter};

pub use confirm::*;
pub use error::*;
pub use result::*;

use crate::chat::DangerousCommand;

mod confirm;
mod error;
mod result;

/// Possible outputs for chat commands.
pub enum CommandOutput<'a> {
    /// Outputs for successful commands that list some result.
    Result(CommandResultOutput<'a>),

    /// Outputs for dangerous commands that need confirmation.
    Confirm(DangerousCommand<'a>, CommandConfirmOutput<'a>),

    /// Outputs for failed commands.
    Error(CommandErrorOutput<'a>),
}

impl Display for CommandOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandOutput::Result(output) => output.fmt(f),
            CommandOutput::Confirm(_, output) => output.fmt(f),
            CommandOutput::Error(output) => output.fmt(f),
        }
    }
}

/// Truncates a text to not exceed the specified number of chars.
pub(super) fn truncate(text: &str, columns: usize) -> String {
    if text.len() > columns {
        text.chars().take(columns - 1).chain("…".chars()).collect()
    } else {
        text.to_string()
    }
}
