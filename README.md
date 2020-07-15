# Steward &emsp; ![version][badge-version] ![date][badge-date] ![new commits][badge-commits] ![build][badge-build]

[badge-version]: https://img.shields.io/github/v/release/timwie/steward?include_prereleases&style=flat-square
[badge-date]: https://img.shields.io/github/release-date-pre/timwie/steward?style=flat-square
[badge-commits]: https://img.shields.io/github/commits-since/timwie/steward/latest?include_prereleases&label=commits%20since&style=flat-square
[badge-build]: https://img.shields.io/github/workflow/status/timwie/steward/CI?style=flat-square

Steward is a controller for [Trackmania]'s community servers.
Server controllers can interface with the dedicated servers and game clients
to add functionality on top of what the game offers by default.

#### Test Server
TBD

#### Screenshots
TBD

#### Contents
- [Features](#features)
- [Project Ambitions](#project-ambitions)
- [Getting Started](#getting-started)
- [Related Projects](#related-projects)
- [License](#license)

#### See also
- The version history is available in [CHANGELOG.md](CHANGELOG.md).
- There are instructions for contributors in [CONTRIBUTING.md](CONTRIBUTING.md).
- Example configurations for server & controller can be found in the
  [config/](/config) directory.
- A guide to deploy this controller with Docker can be found in the
  [docker/](/docker) directory.

<br>

## Features
- [x] **Map Rankings**
 - Compare your personal best in a ranking of local records.
 - Try to beat the best records on every map.
- [x] **Server Ranking**
 - Set top records on every map to rise in the server ranking.
 - You will get updates of your progression at the end of every race.
- [x] **Playlist**
 - Bring up the list of maps to see all of your record rankings.
- [x] **Map Preferences**
 - Cast your vote on a map to make either it more or less likely
   to be played whenever you connect to the server.
 - Open the playlist to find maps you want to improve your record on,
   and *pick* them.
 - If you'd rather skip a map, you can *veto* it.
 - If you do not like a map at all, vote to *remove* it.

You can find a more detailed list of all features [here](CHANGELOG.md).

<br>

## Project Ambitions
- This project was started in anticipation of the new Trackmania game in 2020.
  Other controllers (see [Related Projects](#related-projects)) at this time
  typically integrate with any game, environment and mode within [ManiaPlanet],
  which includes the previous iteration of Trackmania.
  
- At least for now, **this controller targets only the classic *Time Attack* mode of
  the new Trackmania.**
  
- Support for TM² servers or other game modes - in any of the games -
  is not a goal of this project.
  
- Within this single game mode, the controller is supposed to: 
  - Encourage players to set records on all maps on the server.
  - Help players see on which maps and in which sectors they can improve.
  - Let players influence the map queue, so that they it's more likely they get
    to play the maps they want to play.
  - Provide admins with tools to keep the map rotation fresh.
  
- The design of new user interfaces should be optimized for couch gaming:
  - A gamepad must suffice to navigate. Do not require use of a mouse.
  - Readability must be ensured for players sitting a bit further from the
    (albeit larger) screen.
  - Present information in widgets, instead of relying on chat messages.
  - Do not hide common functionality behind chat commands.

<br>

## Getting Started
Most of this setup can be automated by using Docker -
instructions are available in the [docker/](/docker) directory.

#### 1. Prerequisites
- Download the controller & an example config [here](https://github.com/timwie/steward/releases).
- Download the [dedicated server].
- Setup a [PostgreSQL server]. You can freely choose username, database, etc.

#### 2. Server Configuration
- Extract both the dedicated server and the example config.
- Copy the `UserData/` contents of the example config into the `UserData/` directory of the dedicated server.
- Update the server config in `UserData/Config/dedicated_cfg.txt`. Refer to [this tutorial].

#### 3. Controller Configuration
- Update the controller config in `steward.toml` (you can place this file anywhere).
- Use an appropriate connection string for `postgres_connection`, f.e. `host=127.0.0.1 user=postgres password=123`.
- The `<authorization_levels>` setting in the server config must match `super_admin_name/pw` in the controller config.
- The `<xmlrpc_port>` setting in the server config must match `rpc_address` in the controller config.
  For example, use address `127.0.0.1:5000` if you choose port 5000.

#### 4. Firewall Configuration
- The following ports need to be opened in your firewall/router settings:
  - Server port (default 2350): TCP & UDP
  - Server P2P port (default 3450): TCP & UDP
- The XML-RPC port (default 5000) should **not** be open to the public.

#### 5. Launching
- You can choose to launch the controller before or after the server.
- Start the server like this:
```
$ ./TrackmaniaServer /game_settings=MatchSettings/maplist.txt /dedicated_cfg=dedicated_cfg.txt
```
- Start the controller like this:
```
$ export STEWARD_CONFIG=/your/path/steward.toml # you can also use an .env file
$ ./steward
```

#### Admin Commands
- Make sure you add your own login to the `super_admin_whitelist` in the controller config.
- List available commands by typing `/help` into the chat in-game.

#### Supervision
- The controller will not try to recover when encountering errors.
  To be on the safe side, you should restart the process automatically.
- The controller logs to `stderr`. Usually, you want to redirect that output to files.
- It's a good idea to do regular backups. Since maps and replays are embedded into
  the Postgres database, you only need to backup the latter. 

#### Upgrading
- You can check for new releases using the `/info` command.
- If the [changelog](CHANGELOG.md) does not state otherwise,
  you can simply exchange the `steward` executable to upgrade to a newer version.
- New versions may alter the database schema on launch.
  I would recommend to create a database backup beforehand, just in case.

#### Multiple Instances
- If you launch several dedicated servers, they will use different ports.
  A second instance f.e. would use ports 2351, 3451 and 5001, and so on.
- This means that you cannot use the same `steward.toml` config file for
  every instance. You have to provide the correct port in the `rpc_address`
  setting.
- You also have to choose a different `postgres_connection`, to not use the
  same database for multiple instances.

<br>

## Related Projects
Here are some more "general-purpose" server controllers that are less opinionated,
and offer a plugin architecture that is arguably easier to extend:
- [PyPlanet]
- [EvoSC]
- [ManiaControl]

### Acknowledgements
- [belak/serde-xmlrpc]: This repository was a great reference
  for parsing & composing XML-RPC data. There are some game
  specifics that ultimately made it easier to include an implementation
  here, instead of using a library.

<br>

## License
Distributed under the MIT License. See `LICENSE` for more information.


[Dedicated Server]: http://files.v04.maniaplanet.com/server/TrackmaniaServer_Latest.zip
[PostgreSQL server]: https://www.postgresql.org/download/
[this tutorial]: https://forums.ubisoft.com/showthread.php/2242192-Tutorial-Trackmania-2020-Dedicated-Server

[Issues]: /issues

[Trackmania]: https://trackmania.com/
[ManiaPlanet]: https://www.maniaplanet.com/
[Dedimania]: http://dedimania.net/tmstats/
[Exchange]: https://trackmania.exchange/

[ManiaControl]: https://github.com/ManiaControl/ManiaControl
[PyPlanet]: https://github.com/PyPlanet/PyPlanet
[EvoSC]: https://github.com/EvoTM/EvoSC

[belak/serde-xmlrpc]: https://github.com/belak/serde-xmlrpc
