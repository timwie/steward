use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use gbx::{SCRIPT_API_VERSION, SERVER_API_VERSION};

use crate::config::Config;
use crate::database::Database;
use crate::ingame::{MapInfo, ModeInfo, ModeOptions, Server, ServerInfo, ServerOptions};

/// Runs everything that needs to run at startup.
pub async fn prepare(server: &Arc<dyn Server>, db: &Arc<dyn Database>, config: &Config) {
    log::debug!("using server API version '{}'", SERVER_API_VERSION);
    log::debug!("using script API version '{}'", SCRIPT_API_VERSION);

    prepare_rpc(server, config).await;
    prepare_server(server).await;
    prepare_mode(server, config).await;

    db.migrate().await.expect("failed to migrate database");
    prepare_playlist(server, db).await;
}

/// Make sure that we can make server calls, and receive server callbacks.
async fn prepare_rpc(server: &Arc<dyn Server>, config: &Config) {
    log::debug!("prepare XML-RPC...");
    server
        .authenticate(&config.super_admin_name, &config.super_admin_pw)
        .await;
    server.enable_callbacks().await;
    server.set_api_version().await;
    server.enable_manual_chat_routing().await;

    // Whenever the controller is shut down, it won't remove widgets for players,
    // so it is possible that we need to clear them first. Especially helpful
    // during development.
    server.clear_manialinks().await;
}

/// Check server compatibility and override some server options in the
///  `.../UserData/Config/*.txt` to ensure the functionality of this controller.
async fn prepare_server(server: &Arc<dyn Server>) {
    log::debug!("prepare server...");
    check_server_compat(server.server_info().await);

    let mut server_options = server.server_options().await;
    add_server_option_constraints(&mut server_options);
    log::info!("using server options:");
    log::info!("{:?}", &server_options);
    server.set_server_options(&server_options).await;
}

/// There are a few server options that will be overridden
/// to ensure the functionality of this controller.
fn add_server_option_constraints(options: &mut ServerOptions) {
    // Disallow votes: we want to handle restart votes ourselves.
    // Unfortunately, this overrides the ratio for every vote,
    // but I suppose they are not commonly used anyway.
    options.call_vote_ratio = -1.;

    // Prefer saving proper validation or ghost replays on demand.
    options.auto_save_replays = false;

    // We will save record replays in the database whenever
    // they are set, no need to save best runs for every match.
    options.auto_save_validation_replays = false;

    // New players will be announced in the chat instead.
    options.disable_service_announces = true;

    // Let players keep their slots when switching to spectator,
    // but implement a mechanism that removes spectator's player slots,
    // if they spectate too long.
    options.keep_player_slots = true;
}

/// Log the server info if the build is not exactly the one that was developed on.
/// For newer builds, this should not cause incompatibilities, but it might still
/// be good to be aware of them.
fn check_server_compat(info: ServerInfo) {
    static SERVER_KNOWN_NAME: &str = "ManiaPlanet";
    static SERVER_KNOWN_VERSION: &str = "3.3.0";
    static SERVER_KNOWN_BUILD: &str = "2019-10-23_20_00";

    if info.name != SERVER_KNOWN_NAME {
        log::warn!("server is not a ManiaPlanet server:");
        log::warn!("{:?}", info);
        return;
    }
    if info.version != SERVER_KNOWN_VERSION || info.build != SERVER_KNOWN_BUILD {
        log::warn!("server has an unexpected build:");
        log::warn!("{:?}", info);
        return;
    }
    if !info.title_id.starts_with("TM") || !info.title_id.ends_with("@nadeo") {
        log::warn!(
            "server does not play a Nadeo Trackmania title: {}",
            &info.title_id
        );
    }
}

/// Set & configure the game mode.
/// Overwrite the default `<ui_properties>`.
async fn prepare_mode(server: &Arc<dyn Server>, config: &Config) {
    static TA_SCRIPT_TEXT: &str = include_str!("../res/TimeAttack.Script.txt");

    log::debug!("prepare game mode...");

    // Change game mode if we have to.
    if !check_mode_compat(server.mode().await) {
        log::info!("replacing game mode with bundled Time Attack script");
        server
            .set_mode(TA_SCRIPT_TEXT)
            .await
            .expect("failed to set mode script");
    }
    log::info!("using mode:");
    log::info!("{:?}", server.mode().await);

    let mut mode_options = server.mode_options().await;
    add_mode_option_constraints(&config, &mut mode_options);
    log::info!("using mode options:");
    log::info!("{:?}", &mode_options);
    server.set_mode_options(&mode_options).await;

    let ui_properties_xml = include_str!("../res/UiProperties.xml");
    server.set_ui_properties(&ui_properties_xml).await;
}

/// Check the server's game mode, and return `True` if it is compatible
/// with this controller.
///
/// Log the mode info if it's not exactly the version that was developed on.
/// It's unlikely that we get incompatibilities with newer Time Attack versions,
/// but it might still be good to be aware of them.
fn check_mode_compat(info: ModeInfo) -> bool {
    if info.file_name != TA_SCRIPT && info.file_name != CUSTOM_SCRIPT {
        log::warn!("mode is not Time Attack!");
        log::warn!("{:?}", info);
        return false;
    }
    if !info
        .compatible_map_types
        .split(',')
        .any(|typ| typ == TA_MAP_TYPE)
    {
        log::warn!("mode does not support Race map type!");
        log::warn!("{:?}", info);
        return false;
    }
    if info.version != TA_KNOWN_VERSION {
        log::warn!("mode has different version '{}'", info.version);
    }
    true
}

/// Set mode options according to this controller's config file.
fn add_mode_option_constraints(config: &Config, options: &mut ModeOptions) {
    options.chat_time_secs = config.outro_duration_secs as i32;
    options.race_duration_secs = config.race_duration_secs as i32;
}

/// When starting a server, there are three sources for a map list:
/// - the controller's database
/// - the playlist provided when launching the server
///   (`/game_settings=MatchSettings/maplist.txt`)
/// - the actual map files in `.../UserData/Maps/`
///
/// To remove any inconsistencies,
/// - Add any file in `.../UserData/Maps/` to the database.
///   - If the map is new, it will be enabled by default.
/// - Restore deleted map files with the copies in the database.
/// - Set the playlist to include exactly those maps, that are enabled in the database.
/// - Overwrite the match settings in `.../UserData/Maps/MatchSettings/maplist.txt`.
///
/// This has some other advantages:
/// - If the server plays a map, we can be certain it's in the database.
/// - Every map in the database can be added to the playlist,
///   even when its file has been deleted.
///
/// # Limitations
/// We assume that we have access to the server's filesystem, which
/// would prevent running server & controller in separate containers.
///
/// # Panics
/// This function panics if there is no enabled map. Disabling all maps
/// through this controller should be prevented, but it is still possible
/// when deleting files. To fix this, add a new map in `.../UserData/Maps/`.
async fn prepare_playlist(server: &Arc<dyn Server>, db: &Arc<dyn Database>) {
    log::debug!("prepare playist...");

    fs_maps_to_db(server, db).await;
    db_maps_to_fs(server, db).await;
    db_maps_to_match_settings(server, db).await;
}

/// Add every map in the `.../UserData/Maps/` directory to the database.
/// Disable maps in the database that had their files removed.
///
/// New maps should be enabled, and old maps should have their file name updated
/// in case it changed.
async fn fs_maps_to_db(server: &Arc<dyn Server>, db: &Arc<dyn Database>) {
    let maps_dir = server.user_data_dir().await.join("Maps");

    let map_file_paths: Vec<PathBuf> = fs::read_dir(&maps_dir)
        .expect("failed to read map directory")
        .filter_map(|entry| {
            let path = entry.expect("failed to read map directory").path();
            if let Some("Gbx") = path.extension().and_then(OsStr::to_str) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    let map_file_names: Vec<&str> = map_file_paths
        .iter()
        .filter_map(|path| path.file_name())
        .filter_map(|name| name.to_str())
        .collect();

    // set playlist to all maps that have files
    log::debug!("local map files: {:?}", &map_file_names);
    server.playlist_add_all(map_file_names).await;

    // now we can query map infos
    let map_infos: Vec<MapInfo> = server
        .playlist()
        .await
        .into_iter()
        .filter(|info| !info.is_campaign_map())
        .collect();
    log::debug!("local map infos: {:?}", &map_infos);

    // Insert new maps & update file paths of those already in the database.
    for info in map_infos.iter() {
        let map_file = maps_dir.join(&info.file_name);

        // At this point, the playlist can actually still contain maps
        // that had their file deleted. Ignore those.
        if !map_file.is_file() {
            continue;
        }

        let map_data = read_to_bytes(&map_file).expect("failed to read map file");
        let is_new_map = db
            .map(&info.uid)
            .await
            .expect("failed to load map")
            .is_none();
        db.upsert_map(info, map_data)
            .await
            .expect("failed to upsert map");
        if is_new_map {
            log::info!("found new map: {:?}", &info);
        }
    }
}

/// Add every map in the database to the file system, in case
/// they have been deleted.
///
/// Panics if the file could not be written.
async fn db_maps_to_fs(server: &Arc<dyn Server>, db: &Arc<dyn Database>) {
    let maps_dir = server.user_data_dir().await.join("Maps");

    let restorable_maps = db.map_files().await.expect("failed to fetch db maps");

    // Restore map files that have been removed from the file system.
    for map in restorable_maps.iter() {
        let map_path = maps_dir.join(&map.metadata.file_name);
        if !map_path.is_file() {
            log::info!("restore map file: {:?}", map_path);
            fs::write(&map_path, &map.data).expect("failed to restore map file");
        }
    }
}

/// Write all maps in the database to `.../UserData/MatchSettings/maplist.txt`.
///
/// Panic if there are no maps in the database.
async fn db_maps_to_match_settings(server: &Arc<dyn Server>, db: &Arc<dyn Database>) {
    const MATCH_SETTINGS_PATH: &str = "MatchSettings/maplist.txt";

    let db_maps = db.maps().await.expect("failed to fetch maps");

    let is_empty_playlist = db_maps.iter().all(|map| !map.in_playlist);

    if is_empty_playlist {
        log::info!("playlist is empty - a random map will be added");
        let map = db_maps
            .first()
            .expect("map list is empty - you should add a new map to /UserData/Maps/");
        db.playlist_add(&map.uid)
            .await
            .expect("failed to enable map");
    }

    let playlist_files: Vec<&str> = db_maps.iter().map(|map| map.file_name.as_ref()).collect();

    log::info!("using playlist:");
    log::info!("{:?}", &playlist_files);

    // Put all maps in the playlist, regardless whether enabled or not
    server.playlist_replace(playlist_files).await;

    // Overwrite playlist in the match settings file
    server.playlist_save(MATCH_SETTINGS_PATH).await;

    let match_settings_file = server
        .user_data_dir()
        .await
        .join("Maps")
        .join(MATCH_SETTINGS_PATH);

    // If we use a custom script, the match settings will contain
    // "<script_name><in-development></script_name>", which will prevent
    // restarting the server, as it's not a valid name. We will replace
    // it with the Time Attack mode.
    let match_settings_xml =
        fs::read_to_string(&match_settings_file).expect("failed to read match settings file");
    let match_settings_xml = match_settings_xml.replace(CUSTOM_SCRIPT, TA_SCRIPT);
    fs::write(match_settings_file, &match_settings_xml)
        .expect("failed to write match settings file");

    log::debug!("using match settings:");
    log::debug!("{}", match_settings_xml);
}

fn read_to_bytes(file_path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let metadata = fs::metadata(file_path)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer)?;
    Ok(buffer)
}

static CUSTOM_SCRIPT: &str = "<in-development>"; // the name if we set the script ourselves

static TA_SCRIPT: &str = "TimeAttack.Script.txt";
static TA_MAP_TYPE: &str = "Race";
static TA_KNOWN_VERSION: &str = "2018-05-14";
