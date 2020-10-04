use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use chrono::Utc;

use gbx::file::parse_map_file;

use crate::config::Config;
use crate::constants::{BLACKLIST_FILE, VERSION};
use crate::database::{DatabaseClient, Map, MapQueries};
use crate::network::exchange_id;
use crate::server::{
    Calls, Server, ServerBuildInfo, ServerOptions, SCRIPT_API_VERSION, SERVER_API_VERSION,
};

/// Runs everything that needs to run at startup.
pub async fn on_startup(server: &Server, db: &DatabaseClient, config: &Config) {
    log::debug!("using Steward version '{}'", VERSION.to_string());
    log::debug!("using server API version '{}'", SERVER_API_VERSION);
    log::debug!("using script API version '{}'", SCRIPT_API_VERSION);

    // Authenticate, set API versions, enable callbacks.
    prepare_rpc(server, config).await;

    // Log if server version does not match version used in development.
    check_server_compat(server.server_build_info().await);

    // Override some options in `.../UserData/Config/*.txt` to ensure the
    // functionality of this controller.
    let mut server_options = server.server_options().await;
    add_server_option_constraints(&mut server_options);
    log::info!("using server options:");
    log::info!("{:#?}", &server_options);
    server.set_server_options(&server_options).await;

    // Load the blacklist from disk.
    load_blacklist(server).await;

    // If needed, migrate the database.
    db.migrate().await.expect("failed to migrate database");

    // Sync filesystem and database maps.
    prepare_maps(server, db).await;

    // Whenever the controller is shut down, it won't remove widgets for players,
    // so it's best to clear them here. Especially helpful during development.
    server.clear_manialinks().await;

    // "Clearing" the chat is also helpful during development.
    let empty_lines = std::iter::repeat("\n").take(10).collect::<String>();
    server.chat_send(&empty_lines).await;
}

/// Make sure that we can make server calls, and receive server callbacks.
async fn prepare_rpc(server: &Server, config: &Config) {
    server
        .authenticate(&config.rpc_login, &config.rpc_password)
        .await;
    server.enable_callbacks().await;
    server.set_api_version().await;
    server
        .enable_manual_chat_routing()
        .await
        .expect("another controller is already routing the chat");
}

/// There are a few server options that will be overridden
/// to ensure the functionality of this controller.
fn add_server_option_constraints(options: &mut ServerOptions) {
    // Disallow votes: we want to handle restart votes ourselves.
    // Unfortunately, this overrides the ratio for every vote,
    // but I suppose they are not commonly used anyway.
    options.call_vote_ratio = -1.;

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
fn check_server_compat(info: ServerBuildInfo) {
    const SERVER_KNOWN_VERSION: &str = "3.3.0";
    const SERVER_KNOWN_BUILD: &str = "2020-10-02_20_30";

    if info.name != "Trackmania"
        || info.version != SERVER_KNOWN_VERSION
        || info.version_date != SERVER_KNOWN_BUILD
    {
        log::warn!("server has an unexpected version:");
        log::warn!("{:#?}", info);
    }
}

/// Load the blacklist file, or create it if it doesn't exist yet.
async fn load_blacklist(server: &Server) {
    let blacklist_file = server
        .user_data_dir()
        .await
        .join("Config")
        .join(BLACKLIST_FILE);

    if !blacklist_file.is_file() {
        let empty_list = r#"
        <?xml version="1.0" encoding="utf-8" ?>
        <blacklist>
        </blacklist>
        "#;
        std::fs::write(blacklist_file, empty_list).expect("failed to create blacklist file");
        server
            .save_blacklist(BLACKLIST_FILE)
            .await
            .expect("failed to write empty blacklist file");
    }

    server
        .load_blacklist(BLACKLIST_FILE)
        .await
        .expect("failed to load blacklist file");
}

/// When starting a server, there are two sources for a map list:
/// - the controller's database
/// - the actual map files in `.../UserData/Maps/`
///
/// To remove any inconsistencies,
/// - Add any file in `.../UserData/Maps/` to the database.
/// - Restore deleted map files with the copies in the database.
///
/// Doing this, we ensure that every map in the database can be added to the
/// server's playlist, and that every map in the server's playlist is
/// in the database.
///
/// # Panics
/// This function requires that the controller has access to the server's filesystem,
/// which must be ensured when running them in containers. If map files cannot be read,
/// this function panics.
async fn prepare_maps(server: &Server, db: &DatabaseClient) {
    check_maps(server, db).await;
    check_deleted_maps(server, db).await;
}

/// Add every map in the `.../UserData/Maps/` directory to the database.
///
/// New maps are persisted in the database.
/// We will also try to find their IDs on Trackmania Exchange.
///
/// Old maps will have their file updated in case it changed.
async fn check_maps(server: &Server, db: &DatabaseClient) {
    let maps_dir = server.user_data_dir().await.join("Maps");

    let map_files = map_files_in(&maps_dir);

    // Insert new maps & update file paths of those already in the database.
    for map_file in map_files.into_iter() {
        let map_file_name = map_file
            .strip_prefix(&maps_dir)
            .expect("failed to read map file name")
            .to_str()
            .expect("failed to read map file name");

        upsert_map(db, &map_file, map_file_name).await;
    }
}

fn map_files_in(path: &PathBuf) -> Vec<PathBuf> {
    fs::read_dir(&path)
        .expect("failed to read map directory")
        .flat_map(|entry| {
            let path = entry.expect("failed to read map directory").path();
            if path.is_dir() {
                map_files_in(&path)
            } else if let Some("Gbx") = path.extension().and_then(OsStr::to_str) {
                vec![path]
            } else {
                vec![]
            }
        })
        .collect()
}

fn read_to_bytes(file_path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let metadata = fs::metadata(file_path)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer)?;
    Ok(buffer)
}

async fn upsert_map(db: &DatabaseClient, map_file: &PathBuf, map_file_name: &str) {
    let header = match parse_map_file(&map_file) {
        Ok(header) => header,
        Err(err) => {
            log::error!("failed to read map header in {}: {}", map_file_name, err);
            return;
        }
    };

    let fs_map = Map {
        uid: header.uid,
        file_name: map_file_name.to_string(),
        name: header.name,
        author_login: header.author_login,
        author_display_name: header.author_display_name,
        added_since: Utc::now().naive_utc(),
        author_millis: header.millis_author,
        exchange_id: None,
    };

    let fs_map_data = read_to_bytes(&map_file).expect("failed to read map file");

    let maybe_db_map = db.map(&fs_map.uid).await.expect("failed to load map");

    let is_new_map = maybe_db_map.is_none();

    let mut new_db_map = maybe_db_map.unwrap_or(fs_map);

    // Try to find exchange ID
    if new_db_map.exchange_id.is_none() {
        if let Ok(id) = exchange_id(&new_db_map.uid).await {
            new_db_map.exchange_id = Some(id);
        }
    }

    // FIXME if a new map has the same file name as a deleted map, this will violate the unique constraint
    //  => this is likely if a map was updated, but still has the same name
    db.upsert_map(&new_db_map, fs_map_data)
        .await
        .expect("failed to upsert map");

    if is_new_map {
        log::info!("found new map: {:#?}", &new_db_map);
    }
}

/// For every map in the database that was removed from the file system, restore their file.
///
/// Panics if the file could not be written.
async fn check_deleted_maps(server: &Server, db: &DatabaseClient) {
    let maps_dir = server.user_data_dir().await.join("Maps");

    let restorable_maps = db.maps(vec![]).await.expect("failed to fetch db maps");

    // Restore map files that have been removed from the file system.
    for map in restorable_maps.iter() {
        let map_path = maps_dir.join(&map.file_name);

        if !map_path.is_file() {
            let map_data = db
                .map_file(&map.uid)
                .await
                .expect("failed to restore map file")
                .expect("failed to restore map file");

            log::info!("restore deleted map file: {:#?}", map_path);
            fs::write(&map_path, &map_data).expect("failed to restore map file");
        }
    }
}
