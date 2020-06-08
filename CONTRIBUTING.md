# Contributing

### Players and Admins
- Feel free to open a new [Issue] if you have problems or ideas.
- Keep in mind that requested features might be beyond [the scope] of this project.
- If you have found a bug,
  - try to find a way to replicate it consistenly ("if you do X and Y, this happens")
  - provide the controller version (type `/info` in chat)
  - provide the controller log if possible (use `RUST_LOG=steward=debug,gbx=debug`
    if you can replicate the issue)

### Developers
If you are looking to contribute code, check out the sections below:

- [Prerequisites](#prerequisites)
- [Controllers 101](#controllers-101)
- [Debugging](#debugging)
- [References](#references)
- [Glossary](#glossary)
- [Pull Requests](#pull-requests)

<br>

## Prerequisites
These steps will allow you to run a server and a controller instance on Windows,
so that you can join the server on the same machine.

1. Install the Rust toolchain, f.e. with [rustup].
2. Download the [Dedicated Server].
3. Setup a [PostgreSQL server].
4. Use the example config in the [config](/config) directory:
   - Copy the `UserData/` contents of the example config into the `UserData/` directory of the dedicated server.
   - Update the controller config in `steward.toml` accordingly.
5. Start the dedicated server (f.e. in a PowerShell prompt).
```
$ cd <your path>/ManiaplanetServer_Latest/
$ .\ManiaPlanetServer /nodaemon /lan /game_settings=MatchSettings/maplist.txt /dedicated_cfg=dedicated_cfg.txt
```
6. Start the controller (f.e. in a PowerShell prompt).
```
$ cd <repository>/
$ set STEWARD_CONFIG=.../steward.toml
$ cargo run
```
7. Start the game, select 'Local Play', and join the server.


<br>

## Controllers 101
This section gives a brief introduction to the API that lets us
interact with the game's dedicated server. 
As far as I can tell, most of what we use for this
controller has been available since 2012, while some parts of the interface
date back to the mid-2000s.

- The controller sends Remote Procedure Calls (RPC) to the game server,
  and receives a response in return.
  - You can, for example, request information about the current map.
  - Or, you can tell the server to change maps.
- There are also RPCs from the game server to the controller. These *callbacks*
  notify the controller of almost any event happening on the game server:
  - General callbacks, f.e. when a player connects.
  - Mode-specific callbacks, f.e. when a player crosses a checkpoint in Trackmania.
- The controller can add in-game UI by sending *Manialinks* and *ManiaScript*,
  which could be seen as the game's own flavour of HTML and JavaScript.
- ManiaScripts add another layer to controllers that runs directly on each player's
  game client.
- All this gives us two ways to let players interact with the controller:
  - With ManiaScript, we can trigger an *action* that, if needed, can carry a
    JSON payload, and is received by the controller via a callback.
  - Also received via callbacks are chat messages. Parsing these as commands
    is ideal for admin features that do not necessarily need a visual
    component.
- There are a few concerns to keep in mind when designing UI widgets:
  - It's not possible to send data back to widgets, which means
    they cannot be updated, but only replaced.
  - An exception are widgets that can be updated solely with in-game data:
    a widget displaying the number of players f.e. can find that information
    using ManiaScript.
  - The overhead of updating widgets makes splitting the UI
    into smaller widgets a good idea, since we don't want to re-send
    more data than we have to.
  - But: widgets cannot communicate, or share code with other widgets.
  - One consequence of this is that we cannot properly support gamepad interaction for more than
    one widget at a time: every widget will receive the key inputs, but since there is
    no shared state, it would be too complicated to track the focus across multiple
    widgets.

The Rust bindings for this API are implemented in the [`gbx` crate].

The controller implementation, at least at the top-level, is simply a loop that
receives and processes callbacks.

<br>

## Debugging
First, make sure to enable logging: `$ export RUST_LOG=info,steward=debug,gbx=debug`

Any panic that crashes the program - apart from losing connection to the game server -
is a bug. The following list contains many of the potential runtime errors when developing:
  - Using `.unwrap()` on `Err` or `None`.
  
  - **SQL errors:** Not using an ORM means we do not get any compile-time
    checks. You can try queries inside the `pgsql` shell, but that still leaves
    errors caused when binding parameters, or accessing the output data.
    
  - **XML-RPC faults:** As there is no clear documentation on the possible
    error cases of RPC methods, we might try to `.unwrap()` faults
    when not expecting them.
    
  - **Template errors**: Rendering widgets will `panic` if the templates or the input
    context are not valid.
    
  - **ManiaScript errors:** You can press `Ctrl + ~` to display ManiaScript logs
    and errors in-game. Some errors are detected early, when the scripts are compiled. 
    
  - **Deadlocks**: We often use an `RwLock`, which allows us to keep an internal
    state, that can be read by multiple readers, but is locked whenever it is being
    modified. You can introduce a deadlock if you get a mutable reference with 
    `state.write().await`, and then call `state.read().await` later, while still holding
    the `RwLockWriteGuard`. This might happen f.e. when you use a helper function that reads
    something from the state, inside a function that alters the state.
    
  - **Wrong port**: If you encounter `Fault { code: -1000, msg: "Not in script mode." }`, chances
    are that you started the game client before starting the server. In that case,
    the server will listen at port `5001` instead of `5000`, which is reserved by the game
    client.

<br>

## References
- [Dedicated Server Documentation]
- [Dedicated Server Forum]
- [Server Settings for Mode Scripts]
- [XML-RPC Methods]
- [XML-RPC Callbacks]
- [XML-RPC Methods & Callbacks for Mode Scripts]
- [XML-RPC Client Example]
- [ManiaPlanet Mode Scripts]
- [Manialink Reference]
- [ManiaScript Reference]
- [Trackmania Race API for Manialink Scripts]
- [In-game Text Formatting]
- [Dedimania API]
- [Dedimania Forum]
- [ManiaExchange API]


<br>

## Glossary

| Term           | Meaning                                                                               |
|:---------------|:--------------------------------------------------------------------------------------|
| Action         | Refers to events that are triggered by players interacting with widgets.              |
| Callbacks      | Refers to remote procedure calls sent by the game server, executed by the controller. |
| Calls          | Refers to remote procedure calls sent by the controller, executed by the game server. |
| Command        | Refers to commands written by players in the in-game chat.                            |
| Config         | • *Server config* refers to the XML config in `/UserData/Config/*.txt`, passed to the `/dedicated_cfg` launcher option. Some options can will be overridden by the controller.<br> • *Controller config* refers to the TOML config listed in the `STEWARD_CONFIG` environment variable. |
| [Dedimania]    | A popular database for global records.                                                |
| [Exchange]     | A popular website for sharing maps.                                                   |
| Intro & Outro  | • Refers to the stages before and after a race, respectively.<br> • During the intro, the server briefly waits for players to load, and the map's MediaTracker intro is shown as a cutscene.<br> • During the outro, the game gives players some time to chat, and to inspect the scores. |
| Manialink      | In-game "web pages", written in XML with specific tags.                               |
| ManiaScript    | In-game scripting language, that can, among other things, be used to make Manialinks interactive. |
| Match Settings | These settings include game mode, mode script settings, and playlist. Initialized with the file passed to the `/game_settings` launcher option, then modified by the controller, overriding the file. |
| MediaTracker   | In-game editor for map authors that can add effects to intros & outros of maps, as well as during runs and replays. |
| Mode Script    | The ManiaScript that implements game mode logic, and mode-specific XML-RPC methods & callbacks. |
| Playlist       | Refers to the list of maps that are played on the server. Initialized with the match settings, then modified by the controller. |
| Queue          | Refers to an ordering of the playlist, that decides which maps are played when.       |
| Race           | Refers to the race of connected players on a single map, within a single time limit.  |
| Rank           | • *Race rank* refers to a player's ranking during the current race.<br> • *Map rank* refers to the rank of a player's personal best in the ranking of all records on map.<br> • *Server rank* refers to rank calculated over all maps, where players with many top records gain top ranks. |
| Record         | • The *Personal Best* (PB) of a player is their best record on a given map.<br> • *Local records* refers to records that were set on this specific server. <br> • *Global records* refers to records across all servers.  |
| Replay         | • *Validation replays* contain only very basic information about a player's run.<br> • *Ghost replays* allow you to playback a run, to observe or race against it.<br> • Both replays contain the map as well. Ghost replays are much larger, primarily because it contains calculated lightmaps for the map, giving it better ambient lighting. |
| Run            | • Refers to a drive from start to finish, without respawns to the starting line.<br>• *Run outro* refers to the duration between finishing a run, and starting the next. |
| Sector         | Refers to the sections of a run in between start, checkpoint, and finish blocks.      |
| Spectator      | • Every server has a number of player slots, and spectator slots. <br> • A *spectator* is a player spectating another player, while still occupying a player slot, allowing them to switch back to racing at any time.<br> • *Pure spectators* only have a spectator slot, and must wait for a player slot to free up in case they want to join the race. |
| Widget         | Refers to additional in-game UI implemented with Manialinks and ManiaScripts.         |


<br>

## Pull Requests
Before you start to work on anything major, please create an issue,
or leave a comment on an exisiting one.

To get your changes merged...
1. Fork the Project.
2. Create your branch: `git checkout -b <new-branch> <existing-branch>`
   (you can most likely just branch off `master`)
3. Make your changes.
4. When you have pushed all changes to the fork, open a [Pull Request].
5. If requested, keep pushing changes to your fork, the PR will be
   updated automatically.
6. Once the pull request is approved and merged you can pull the changes
   from `upstream` to your local repo and delete your extra branch(es).

Remember to...
- run `cargo check`
- run `cargo test`
- run `cargo fmt`
- run `cargo clippy`
- update other documents (README, CHANGELOG, this document, etc.) if required
- test new or changed behavior on a local server (you don't have to write unit tests)

[the scope]: /README.md#project-ambitions
[`gbx` crate]: /gbx/
[Issue]: https://github.com/timwie/steward/issues/new
[Pull Request]: https://github.com/timwie/steward/compare

[Dedimania]: http://dedimania.net/tmstats/
[Exchange]: https://trackmania.exchange/

[rustup]: https://rustup.rs/
[Dedicated Server]: http://files.v04.maniaplanet.com/server/ManiaplanetServer_Latest.zip
[PostgreSQL server]: https://www.postgresql.org/download/

[Dedicated Server Documentation]: https://doc.maniaplanet.com/dedicated-server/getting-started
[Dedicated Server Forum]: https://forum.maniaplanet.com/viewforum.php?f=533
[Server Settings for Mode Scripts]: https://doc.maniaplanet.com/dedicated-server/references/settings-list-for-nadeo-gamemodes
[XML-RPC Methods]: https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-methods
[XML-RPC Callbacks]: https://doc.maniaplanet.com/dedicated-server/references/xml-rpc-callbacks
[XML-RPC Methods & Callbacks for Mode Scripts]: https://github.com/maniaplanet/script-xmlrpc/
[XML-RPC Client Example]: https://github.com/maniaplanet/dedicated-server-api
[ManiaPlanet Mode Scripts]: https://github.com/maniaplanet/game-modes
[Manialink Reference]: https://doc.maniaplanet.com/manialink/getting-started
[ManiaScript Reference]: https://doc.maniaplanet.com/maniascript/syntax-basics
[Trackmania Race API for Manialink Scripts]: https://www.uaseco.org/maniascript/2019-10-10/struct_c_tm_ml_script_ingame.html
[In-game Text Formatting]: https://doc.maniaplanet.com/client/text-formatting
[Dedimania API]: http://dedimania.net:8082/Dedimania
[Dedimania Forum]: http://dedimania.net/SITE/forum/viewforum.php?id=17
[ManiaExchange API]: https://api.mania-exchange.com/documents/reference
