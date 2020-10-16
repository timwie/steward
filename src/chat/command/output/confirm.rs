use std::fmt::{Display, Formatter};

/// Outputs for dangerous commands that need confirmation.
pub enum CommandConfirmOutput<'a> {
    /// Tell a super admin that all records for that map will be deleted.
    ///
    /// Output for `/delete map`
    ConfirmMapDeletion { file_name: &'a str },

    /// Tell a super admin that all records for that player will be deleted.
    ///
    /// Output for `/delete player`
    ConfirmPlayerDeletion { login: &'a str },

    /// Tell a super admin that the server will shutdown.
    ///
    /// Output for `/shutdown`
    ConfirmShutdown,
}

impl Display for CommandConfirmOutput<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CommandConfirmOutput::*;

        match self {
            ConfirmMapDeletion { file_name } => writeln!(
                f,
                "Warning: this action will delete map '{}', and all of its records.",
                file_name
            ),

            ConfirmPlayerDeletion { login } => writeln!(
                f,
                "Warning: this action will delete player '{}', and all of their records.",
                login
            ),

            ConfirmShutdown => writeln!(f, "Warning: this will stop the server."),
        }
    }
}
