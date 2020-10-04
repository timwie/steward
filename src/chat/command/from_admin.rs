use std::default::Default;
use std::str::FromStr;

use lazy_static::lazy_static;

use crate::chat::{BadCommandContext, CommandContext, CommandEnum, CommandReference};
use crate::server::ModeScript;

/// Chat commands that can only be executed by admins.
#[derive(Debug, Copy, Clone)]
pub enum AdminCommand<'a> {
    /// Open the config editor.
    ///
    /// Usage: `/config`
    EditConfig,

    /// List the server's maps and their UIDs.
    /// For each map, it should say whether it is in the playlist
    /// or not.
    ///
    /// Usage: `/maps`
    ListMaps,

    /// List the connected players with login and display names.
    ///
    /// Usage: `/players`
    ListPlayers,

    /// Add the map with the given UID to the playlist.
    ///
    /// Usage: `/playlist add <uid>`
    PlaylistAdd { uid: &'a str },

    /// Remove the map with the given UID from the playlist.
    ///
    /// Usage: `/playlist remove <uid>`
    PlaylistRemove { uid: &'a str },

    /// Import a map from `trackmania.exchange`.
    /// The ID is either its ID on the website (a number),
    /// or its UID (from inside the map file).
    ///
    /// Should have the following outcomes:
    /// - Download the map.
    /// - Add it to the database & playlist.
    /// - Queue it as the next map.
    ///
    /// Usage: `/map_import <id/uid>`
    ImportMap { id: &'a str },

    /// End the current race immediately.
    ///
    /// Usage `/skip`
    SkipCurrentMap,

    /// Restart the current map after the race.
    ///
    /// Usage `/restart`
    RestartCurrentMap,

    /// Set the map that will be played after the current one.
    /// Running this command multiple times will queue all maps
    /// in order.
    ///
    /// Usage `/queue <uid>`
    ForceQueue { uid: &'a str },

    /// Add a player to the server's blacklist, and kick them if they are
    /// currently connected.
    ///
    /// Usage: `/blacklist <login>`
    BlacklistAdd { login: &'a str },

    /// Remove a player from the server's blacklist.
    ///
    /// Usage: `/unblacklist <login>`
    BlacklistRemove { login: &'a str },

    /// Pause or unpause the match.
    ///
    /// Usage: `/pause`
    TogglePause,

    /// Extend the current warmup round.
    ///
    /// Usage: `/warmup add <seconds>`
    ExtendWarmup { secs: u64 },

    /// End the current warmup section.
    ///
    /// Usage: `/warmup skip`
    SkipWarmup,
}

lazy_static! {
    static ref ADMIN_COMMANDS: Vec<AdminCommand<'static>> = {
        use AdminCommand::*;
        vec![
            EditConfig,
            ListMaps,
            ListPlayers,
            PlaylistAdd {
                uid: Default::default(),
            },
            PlaylistRemove {
                uid: Default::default(),
            },
            ImportMap {
                id: Default::default(),
            },
            SkipCurrentMap,
            RestartCurrentMap,
            ForceQueue {
                uid: Default::default(),
            },
            BlacklistAdd {
                login: Default::default(),
            },
            BlacklistRemove {
                login: Default::default(),
            },
            TogglePause,
            ExtendWarmup {
                secs: Default::default(),
            },
            SkipWarmup,
        ]
    };
}

impl<'a> CommandEnum<'a> for AdminCommand<'a> {
    fn all() -> &'static Vec<Self> {
        &ADMIN_COMMANDS
    }

    fn parse(chat_message: &'a str) -> Option<Self> {
        use AdminCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/blacklist", login] => Some(BlacklistAdd { login: *login }),
            ["/config"] => Some(EditConfig),
            ["/map_import", id] => Some(ImportMap { id: *id }),
            ["/maps"] => Some(ListMaps),
            ["/pause"] => Some(TogglePause),
            ["/players"] => Some(ListPlayers),
            ["/playlist", "add", uid] => Some(PlaylistAdd { uid: *uid }),
            ["/playlist", "remove", uid] => Some(PlaylistRemove { uid: *uid }),
            ["/queue", uid] => Some(ForceQueue { uid: *uid }),
            ["/restart"] => Some(RestartCurrentMap),
            ["/skip"] => Some(SkipCurrentMap),
            ["/unblacklist", login] => Some(BlacklistRemove { login: *login }),
            ["/warmup", "add", secs] => match u64::from_str(secs) {
                Ok(secs) => Some(ExtendWarmup { secs }),
                Err(_) => None,
            },
            ["/warmup", "skip"] => Some(SkipWarmup),
            _ => None,
        }
    }

    fn check(&self, ctxt: CommandContext) -> Result<(), BadCommandContext> {
        use crate::config::PlayerRole;
        use AdminCommand::*;
        use BadCommandContext::*;
        use ModeScript::*;

        match self {
            _ if ctxt.player_role < PlayerRole::Admin => Err(NoPermission),

            ExtendWarmup { .. } | SkipWarmup if !ctxt.warmup.available => Err(InOtherModes),
            ExtendWarmup { .. } | SkipWarmup if !ctxt.warmup.active => Err(DuringWarmup),

            TogglePause if !ctxt.pause.available => Err(InOtherModes),

            ForceQueue { .. } | SkipCurrentMap | RestartCurrentMap if *ctxt.mode != TimeAttack => {
                Err(InMode(TimeAttack))
            }

            _ => Ok(()),
        }
    }

    fn reference(&self) -> CommandReference {
        use AdminCommand::*;
        match self {
            EditConfig => ("/config", "Open the config editor").into(),
            ListMaps => ("/maps", "List maps in- and outside of the playlist").into(),
            ListPlayers => ("/players", "List connected players' logins and names").into(),
            PlaylistAdd { .. } => ("/playlist add <uid>", "Add a map to the playlist").into(),
            PlaylistRemove { .. } => {
                ("/playlist remove <uid>", "Remove a map from the playlist").into()
            }
            ImportMap { .. } => (
                "/map_import <id/uid>",
                "Import the TMX map with the given id",
            )
                .into(),
            SkipCurrentMap => ("/skip", "Start the next map immediately").into(),
            RestartCurrentMap => ("/restart", "Restart the current map after this race").into(),
            ForceQueue { .. } => ("/queue <uid>", "Select the next map").into(),
            BlacklistAdd { .. } => ("/blacklist <login>", "Add a player to the blacklist").into(),
            BlacklistRemove { .. } => {
                ("/unblacklist <login>", "Remove a player from the blacklist").into()
            }
            TogglePause => ("/pause", "Pause or unpause the current match").into(),
            ExtendWarmup { .. } => {
                ("/warmup add <seconds>", "Extend the current warmup round").into()
            }
            SkipWarmup => ("/warmup skip", "End the current warmup section").into(),
        }
    }
}
