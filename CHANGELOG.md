# Changelog
Version numbers consist of `MAJOR.MINOR.PATCH`:
- `MAJOR`: increased for milestones of some sort
- `MINOR`: increased when features are added or removed
- `PATCH`: increased for bug fix releases

<!-- Added, Changed, Removed, Fixed --> 

## 0.1.0-alpha3
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Added
- **Widget**: Intro
  - This widget is displayed at the start of a map, until the player
    starts their first run.
  - Displays map name & author.
  - Displays the player's current preference for this map.
  - Displays the current distribution of all players' preferences for this map.
  - Displays the player's current rank on this map.

- **Chat**: Super Admin Commands
  - Since these commands are dangerous, they will require clicking a button to
    confirm their execution.
  - `/delete map <uid>` deletes a map that is not in the playlist from the database.
  - `/delete player <login>` deletes a blacklisted player from the database.
  - `/shutdown` shuts down the server.
 
- **Chat**: Admin Commands
  - `/skip` starts the next map immediately.
  - `/restart` restarts the current map after this race.
  - `/queue <uid>` pushes a map to the top of the queue.
  - `/set timelimit <seconds>` changes the race time limit,
    and updates `race_duration_secs` in your config.
  - `/set chattime <seconds>` changes the outro duration at the end of a map,
    and updates `outro_duration_secs` in your config.
  - `/blacklist <login>` adds a player to the server's blacklist.
    The list is persisted in the `blacklist.txt` file created by the server.
  - `/unblacklist <login>` removes a player from the server's blacklist.

- **Chat**: Player Commands
  - `/info` prints information about the server & controller

### Changed
- **Server Rank**: Only records on maps in the playlist count towards players' server rank,
  to prevent that new players are at an unfair disadvantage.

- **Records**: The controller will now store ghost replays for three top records,
  instead of just one.

- **Admin**: Changes to the controller config
  - added `super_admin_whitelist` to list super admins, that have extended admin rights (see new commands)
  - renamed `super_admin_name` as `rpc_login`
  - renamed `super_admin_pw` as `rpc_password`
  - removed `vote_duration_secs`; now uses a fixed ⅔ of `outro_duration_secs`

- **Admin**: The controller will delete ghost replays of records that are not
  in the top three at startup.

### Fixed
- Fixed not updating improved records in the database.

## [0.1.0-alpha2] - 2020-05-23
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Fixed
- Fixed a critical bug that would crash the controller simply by dis- and reconnecting.
- Fixed a bug that lead to comparing incorrect times in the sector times widget.

## [0.1.0-alpha1] - 2020-05-20
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Added
- **Widget: Sector Times**
  - This widget replaces the vanilla UI that compares "Prev" and "Best" runs,
    since it cannot track the player's personal best across multiple races.
  - This widget also replaces the time diff displayed when crossing a checkpoint,
    for the same reason.
  - At the top of the screen, a colored bar indicates whether the player is on course
    to improved their personal best.
  - A first row of sector times lists diffs of a player's PB compared to the top1 record,
    to see which sectors can be improved. They are always visible.
  - A second row of sector times lists diffs of the current run compared to the top record.
    They are visible until the first checkpoint after a respawn, which gives players
    enough time to actually inspect them.
  - Each sector is numbered and colored to show which sector of a current run is better/worse
    than the same sector in a player's PB run.
  - The size of each sector label indicates the length of a sector.

- **Playlist** *(missing in widgets)*
  - The playlist contains the maps that will be played by the server.
    Maps can be added and removed from the playlist, as long as at least one
    map remains.
  - At startup, the controller will synchronize server and controller playlists:
    - Add maps in `/UserData/Maps/*.Map.gbx` to the database, and the playlist.
    - Restore missing map files using copies from the database.
    - Override the playlist in `/UserData/Maps/MatchSettings/maplist.txt`.
  - Admins can modify the playlist using chat commands (see below).

- **Records** *(missing in widgets)*
  - For every map, the controller will store the personal best of every player in the database.
  - Also stored for each record are checkpoint data (time, speed & distance),
    validation replays, and the timestamp of when the record was set.
  - Ghost replays are stored for every new top record.

- **Server Ranks** *(subject to change & missing in widgets)*
  - Players will earn a "win" on each map, for every player
    that has a worse personal best (or none at all).
  - For example, if a player has the 50th rank on a map, and the
    server has had 200 players (with at least one record on any map) in total, they get `199 max wins - 49 losses = 150 wins`
    for that map. How many of those 200 players have actually set a record on that map
    is irrelevant.

- **Map Preferences** *(missing in widgets)*
  - Players will be able to select their preference for each map in the playlist.
  - Possible values are ***Pick***, ***Veto***, or ***Remove***.
  - *Pick* votes by connected players will make a map more likely to be queued;
    *Veto* and *Remove* will make it less likely.
  - *Remove* votes may be used to determine unpopular maps.
  - If the preference is not explicitly set, it defaults to *Pick* for maps that
    a player has not yet played.

- **Map Queue** *(subject to change & missing in widgets)*
  - The next map is decided at the end of each race.
  - Maps have a score, and the map with the highest score will be queued,
    unless the current map will be restarted, or an admin force queued a map.
  - The score of a map increases whenever another map is played.
  - The score increases if a connected player *picked* it, and decreases
    if they *veto* or voted to *remove* it.

- **Restart Votes** *(missing in widgets)*
  - After a race, players can vote for a map restart.
  - An restart requires 50% of players to vote in favor for a first,
    75% for a second, and 100% for a third or subsequent restart.
  - Vanilla votes have been disabled.

- **Chat**: Admin Commands
  - `/help` puts a command reference into the chat.
  - `/map_import <mx id/uid>` imports maps from ManiaExchange.
  - `/maps` lists map names and UIDs.
  - `/playlist add <uid>` adds a map to the playlist.
  - `/playlist remove <uid>` removes a map from the playlist.

- **Widget**: Command Output
  - Output from chat commands will be displayed in a text field,
    so that it can be selected, copied and scrolled.
  - Click anywhere around the text field to hide it.

- **Chat**: Server Messages
  - Player joining
  - Player leaving
  - Changes in top server ranks
  - Changes in top map records
  - Changes in the server playlist
  - Current map
  - Remind players to vote for a restart
  - Remind players to set their map preferences

- **Admin**: Starting the controller requires a valid TOML config.
  - Set the environment variable `STEWARD_CONFIG=<path>`.

- **Admin**: After updating  to a newer version, the controller may run
  automated database migrations at startup.

- **Admin**: If the server is not running the Time Attack mode, it will
  be enforced at startup.

[0.1.0-alpha2]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha2
[0.1.0-alpha1]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha1
