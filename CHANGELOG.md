# Changelog
Version numbers consist of `MAJOR.MINOR.PATCH`:
- `MAJOR`: increased for milestones of some sort
- `MINOR`: increased when features are added or removed
- `PATCH`: increased for bug fix releases

<!-- Updating, Added, Changed, Removed, Fixed, Commits --> 

<br>

## 0.1.0-alpha6
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Updating
- Get the new [default config](https://github.com/timwie/steward/blob/v0.1.0-alpha6/config/).
- Clear your database with `DROP SCHEMA steward CASCADE`.

### Added
- **Admin Commands**:
  - `/pause` toggles a match pause, if supported by the game mode.
  - `/warmup add <seconds>` extends an active warmup round.
  - `/warmup skip` ends the remaining warmup.
  
### Changed
- **Admin**: Playlist Management
  - The controller will no longer override the server's playlist that was
    initialized with the match settings the server is started with.
  - Changes to the playlist in the TimeAttack mode will be saved to
    `MatchSettings/timeattack.txt`. The server can be started with
    these settings to use the most recent TimeAttack playlist.
  - The server can be started with the `MatchSettings/recent.txt`
    settings to use the configuration that was used before
    the server was shutdown.

- **Commands**:
  - The command reference is now displayed in a table, and categorized
    to show which commands cannot be executed by the player, and why. 
  - Players will get useful error messages when trying to execute
    commands that are not available to them.
  - `/info` now lists admins with their display name, rather than their login
  - `/blacklist <login>` was renamed to `/blacklist add <login>`
  - `/unblacklist <login>` was renamed to `/blacklist remove <login>`
  - Added `/blacklist clear` which removes all players from the blacklist.
  
### [Commits](https://github.com/timwie/steward/compare/v0.1.0-alpha5...v0.1.0-alpha6)

<br>

## [0.1.0-alpha5] - 2020-09-01
All `0.1.0-alpha` releases are unstable, and have missing widgets.

This is the first version for the new Trackmania.

### Updating
- Download the new dedicated server.
- Get the new [default config](https://github.com/timwie/steward/blob/v0.1.0-alpha5/config/).
- Clear your database with `DROP SCHEMA steward CASCADE`.

### Changed
- Display map author nick names instead of unreadable logins.
  Since we use the nick name stored in the map file, it will be out of date once
  the author changes their nick name.
- `/map_import` now imports maps from the new Trackmania Exchange.

### Removed
- Replays can no longer be stored in the database.
- **Widget**: Sector times have been removed, since you can race ghosts online
  as well now.

### [Commits](https://github.com/timwie/steward/compare/v0.1.0-alpha4...v0.1.0-alpha5)

<br>

## [0.1.0-alpha4] - 2020-07-01
All `0.1.0-alpha` releases are unstable, and have missing widgets.

This will be the last version built for TM².

### Updating
- The controller config has changed:
  - Refer to the new [default](https://github.com/timwie/steward/blob/v0.1.0-alpha4/config/steward.toml) config.
  - Remove `race_duration_secs`.
  - Add `time_limit_factor`, `time_limit_max_secs` and `time_limit_min_secs`. 
- Clear your database with `DROP SCHEMA steward CASCADE`.
  There are no migrations for `0.1.0-alpha` releases.
  
### Added
- **Widget**: Live Ranks
  - This widget is constantly displayed during a race.
  - Displays the top record time.
  - Displays the difference of the player's personal best to the top record time.
  - Displays the player's current map rank.
  - Displays the player's current server rank.
  
- **Admin**: Dynamic time limits
  - Instead of having the same time limit for every map, it will now be
    set to a duration that depends on the length of the track.
  - Use author time or top record, multiply it by a factor (default 10).
  - Round to a multiple of 30 seconds.
  - Keep it to a minimum (default 5 minutes) and a maximum (default 15 minutes).

- **Chat**: Admin Commands
  - `/config` brings up a text field that can be used to change settings.
    Like the config file, it uses the TOML format.

### Changed
- **Widget**: Sector Times
  - Sectors may now be longer (f.e. spreading across two checkpoints),
    to prevent having too many sectors on longer tracks.
  - Entering a new sector is indicated below the CP diff in the center of the screen.
  - Removed map & author name next to sector diffs.
  - Removed background behind sector labels.

### Removed
- Commands `/set timelimit` and `/set chattime` are removed in favor of `/config`. 

### Fixed
- Fixed a crash that occurred when setting a record while crossing at least one
  checkpoint backwards.

### [Commits](https://github.com/timwie/steward/compare/v0.1.0-alpha3...v0.1.0-alpha4)

<br>

## [0.1.0-alpha3] - 2020-06-09
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Updating
- The controller config has changed:
  - Refer to the new [default](https://github.com/timwie/steward/blob/v0.1.0-alpha3/config/steward.toml) config. - Use `super_admin_whitelist` to list super admins, that have extended admin rights (see new commands).
  - Rename `super_admin_name` to `rpc_login`.
  - Rename `super_admin_pw` to `rpc_password`.
  - Remove `vote_duration_secs`. The vote duration is now a fixed ⅔ of `outro_duration_secs`.

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
  - `/players` lists logins and nicknames of connected players.
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
- **Widget**: Sector Times
  - Display checkpoint diff of current run vs personal best in the center of the screen.

- **Server Ranks**: Only records on maps in the playlist count towards players' server rank,
  to prevent that new players are at an unfair disadvantage.

- **Records**: The controller will now store ghost replays for three top records,
  instead of just one.

- **Admin**: The controller will delete ghost replays of records that are not
  in the top three at startup.

### Fixed
- Fixed not updating improved records in the database.


### [Commits](https://github.com/timwie/steward/compare/v0.1.0-alpha2...v0.1.0-alpha3)
<br>

## [0.1.0-alpha2] - 2020-05-23
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Fixed
- Fixed a critical bug that would crash the controller simply by dis- and reconnecting.
- Fixed a bug that lead to comparing incorrect times in the sector times widget.

### [Commits](https://github.com/timwie/steward/compare/v0.1.0-alpha1...v0.1.0-alpha2)

<br>

## [0.1.0-alpha1] - 2020-05-20
All `0.1.0-alpha` releases are unstable, and have missing widgets.

### Added
- **Widget**: Sector Times
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

[0.1.0-alpha1]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha1
[0.1.0-alpha2]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha2
[0.1.0-alpha3]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha3
[0.1.0-alpha4]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha4
[0.1.0-alpha5]: https://github.com/timwie/steward/releases/tag/v0.1.0-alpha5
