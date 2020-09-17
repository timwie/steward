use std::str::FromStr;

/// Chat commands that can only be executed by admins.
#[derive(Debug)]
pub enum AdminCommand<'a> {
    /// Print a reference of available admin commands.
    ///
    /// Usage: `/help`
    Help,

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

    /// List the connected players with login and nickname.
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

impl AdminCommand<'_> {
    /// Parse an admin command.
    pub fn from(chat_message: &str) -> Option<AdminCommand> {
        use AdminCommand::*;

        let parts: Vec<&str> = chat_message.split_whitespace().collect();

        match &parts[..] {
            ["/blacklist", login] => Some(BlacklistAdd { login: *login }),
            ["/config"] => Some(EditConfig),
            ["/help"] => Some(Help),
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
}

/// Admin command reference that can be printed in-game.
pub(in crate::chat) const ADMIN_COMMAND_REFERENCE: &str = "
/config     Open the config editor.

/maps        List the server's maps and their UIDs.
/players     List the connected players with login and nickname.

/map_import <id/uid>       Import the trackmania.exchange map with the given id.
/playlist add <uid>        Add the specified map to the playlist.
/playlist remove <uid>     Remove the specified map from the playlist.

/skip            Start the next map immediately.
/restart         Restart the current map after this race.
/queue <uid>     Set the map that will be played after the current one.

/pause     Pause or unpause the current match, if supported by the game mode.

/warmup add <seconds>     Extend the current warmup round.
/warmup skip              End the current warmup section.

/blacklist <login>       Add a player to the server's blacklist.
/unblacklist <login>     Remove a player from the server's blacklist.
";
