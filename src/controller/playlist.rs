use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::{RwLock, RwLockReadGuard};

use gbx::file::parse_map_file;

use crate::chat::PlaylistCommandError;
use crate::controller::LiveConfig;
use crate::database::{DatabaseClient, Map, MapQueries};
use crate::event::PlaylistDiff;
use crate::network::{exchange_map, ExchangeError};
use crate::server::{Calls, ModeCalls, Server};

/// Use to lookup the current playlist, and the map that is currently being played.
#[async_trait]
pub trait LivePlaylist: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, PlaylistState>;

    /// The playlist index of the current map.
    async fn current_index(&self) -> Option<usize> {
        self.lock().await.current_index
    }

    /// The map that is currently being played.
    async fn current_map(&self) -> Option<Map> {
        self.lock().await.current_map().cloned()
    }

    /// The UID of the map that is currently being played.
    async fn current_map_uid(&self) -> Option<String> {
        self.lock()
            .await
            .current_map()
            .map(|map| map.uid.to_string())
    }

    /// The playlist index of the specified map, or `None` if that
    /// map is not in the playlist.
    async fn index_of(&self, map_uid: &str) -> Option<usize> {
        self.lock().await.index_of(map_uid)
    }

    /// The map at the given playlist index, or `None` if
    /// there is no such index.
    async fn at_index(&self, index: usize) -> Option<Map> {
        self.lock().await.at_index(index).cloned()
    }

    async fn map(&self, uid: &str) -> Option<Map> {
        self.lock()
            .await
            .maps
            .iter()
            .find(|map| map.uid == uid)
            .cloned()
    }
}

pub struct PlaylistState {
    /// All maps in the playlist.
    pub maps: Vec<Map>,

    /// The playlist index of the current map, or `None` if the current map
    /// is not part of the playlist anymore.
    pub current_index: Option<usize>,
}

impl PlaylistState {
    /// The map at the given playlist index, or `None` if
    /// there is no such index.
    pub fn at_index(&self, index: usize) -> Option<&Map> {
        self.maps.get(index)
    }

    /// The playlist index of the specified map, or `None` if that
    /// map is not in the playlist.
    pub fn index_of(&self, map_uid: &str) -> Option<usize> {
        self.maps
            .iter()
            .enumerate()
            .find(|(_, m)| m.uid == map_uid)
            .map(|(idx, _)| idx)
    }

    /// The UID of the map that is currently being played.
    pub fn current_map(&self) -> Option<&Map> {
        self.current_index.and_then(|idx| self.maps.get(idx))
    }
}

#[derive(Clone)]
pub struct PlaylistController {
    state: Arc<RwLock<PlaylistState>>,
    server: Server,
    db: DatabaseClient,
    live_config: Arc<dyn LiveConfig>,
}

impl PlaylistController {
    pub async fn init(
        server: &Server,
        db: &DatabaseClient,
        live_config: &Arc<dyn LiveConfig>,
    ) -> Self {
        let mut playlist = Vec::new();
        for map in server.playlist().await {
            // TODO support playlists with campaign maps
            assert!(!map.is_campaign_map(), "campaign maps are not supported");

            let map = db
                .map(&map.uid)
                .await
                .expect("failed to load playlist map")
                .expect("failed to load playlist map");
            playlist.push(map);
        }

        if playlist.is_empty() {
            panic!("playlist is empty")
        }

        let curr_index = server.playlist_current_index().await;

        // Change map if the current one is not part of the playlist.
        if curr_index.is_none() {
            server.end_map().await.expect("failed to end map");
        }

        let state = PlaylistState {
            maps: playlist,
            current_index: curr_index,
        };

        PlaylistController {
            state: Arc::new(RwLock::new(state)),
            server: server.clone(),
            db: db.clone(),
            live_config: live_config.clone(),
        }
    }

    /// Set the current playlist index to the one of the next map.
    pub async fn set_index(&self, next_index: usize) -> Map {
        let mut playlist_state = self.state.write().await;
        playlist_state.current_index = Some(next_index);
        playlist_state.maps[next_index].clone()
    }

    /// Add the specified map to the server playlist.
    pub async fn add(&self, map_uid: &str) -> Result<PlaylistDiff, PlaylistCommandError> {
        use PlaylistCommandError::*;

        let mut playlist_state = self.state.write().await;

        if playlist_state.index_of(map_uid).is_some() {
            return Err(MapAlreadyAdded);
        }

        let maybe_map = self.db.map(map_uid).await.expect("failed to load map");

        let map = match maybe_map {
            Some(map) => map,
            None => return Err(UnknownUid),
        };

        // 1. add to server playlist
        self.server
            .playlist_add(&map.file_name)
            .await
            .expect("tried to add duplicate map to playlist");

        // 2. add to controller playlist
        playlist_state.maps.push(map.clone());

        log::info!(
            "added '{}' ({}) to the playlist",
            map.name.plain(),
            &map.uid
        );
        Ok(PlaylistDiff::Append(map))
    }

    /// Remove the specified map from the server playlist.
    pub async fn remove(&self, map_uid: &str) -> Result<PlaylistDiff, PlaylistCommandError> {
        use PlaylistCommandError::*;

        let maybe_map = self.db.map(map_uid).await.expect("failed to load map");

        let map = match maybe_map {
            Some(map) => map,
            None => return Err(UnknownUid),
        };

        let mut playlist_state = self.state.write().await;

        let can_disable = playlist_state.maps.iter().any(|map| map.uid != map_uid);

        if !can_disable {
            return Err(EmptyPlaylistDisallowed);
        }

        let map_index = match playlist_state.index_of(map_uid) {
            Some(index) => index,
            None => return Err(MapAlreadyRemoved),
        };

        // 1. remove from server playlist
        self.server
            .playlist_remove(&map.file_name)
            .await
            .expect("cannot remove that map from playlist");

        // 2. remove from controller playlist
        if playlist_state.current_index == Some(map_index) {
            playlist_state.current_index = None;
        }
        playlist_state.maps.remove(map_index);

        log::info!(
            "remove '{}' ({}) from the playlist",
            map.name.plain(),
            &map.uid
        );
        Ok(PlaylistDiff::Remove {
            was_index: map_index,
            map,
        })
    }

    /// Download a map from [trackmania.exchange](https://trackmania.exchange/),
    /// and add it to the playlist.
    ///
    /// The ID is either its ID on the website (a number), or
    /// its UID (encoded in the GBX file's header).
    pub async fn import_map(&self, map_id: &str) -> Result<PlaylistDiff, PlaylistCommandError> {
        use PlaylistCommandError::*;

        let import_map = match exchange_map(map_id).await {
            Ok(import_map) => import_map,
            Err(ExchangeError::UnknownId) | Err(ExchangeError::NotDownloadable) => {
                return Err(UnknownImportId)
            }
            Err(err) => return Err(MapImportFailed(Box::new(err))),
        };

        let is_already_imported = self
            .db
            .map(&import_map.metadata.uid)
            .await
            .expect("failed to lookup map")
            .is_some();
        if is_already_imported {
            return Err(MapAlreadyImported);
        }

        let maps_dir = self.live_config.maps_dir().await;
        let file_name = format!(
            "{}.{}.Map.gbx",
            &import_map.metadata.name.plain(),
            &import_map.metadata.uid
        );
        let file_path = Path::new(&maps_dir).join(&file_name);

        let write_file_res =
            File::create(&file_path).and_then(|mut file| file.write_all(&import_map.data));
        if let Err(err) = write_file_res {
            log::error!("failed to write imported map to disk: {:?}", err);
            return Err(MapImportFailed(Box::new(err)));
        }

        // 1. add to server playlist
        self.server
            .playlist_add(&file_name)
            .await
            .expect("tried to add duplicate map to playlist");

        // 2. add to db
        let header = match parse_map_file(&file_path) {
            Ok(header) => header,
            Err(err) => return Err(MapImportFailed(err.into())),
        };

        let db_map = Map {
            uid: import_map.metadata.uid,
            file_name,
            name: import_map.metadata.name,
            author_login: header.author_login,
            author_display_name: header.author_display_name,
            added_since: Utc::now().naive_utc(),
            author_millis: header.millis_author,
            exchange_id: Some(import_map.metadata.exchange_id),
        };

        self.db
            .upsert_map(&db_map, import_map.data)
            .await
            .expect("failed to insert map into database");

        // 3. add to controller playlist
        let mut playlist_state = self.state.write().await;
        playlist_state.maps.push(db_map.clone());

        log::info!(
            "imported map '{}' ({}) into the playlist",
            db_map.name.plain(),
            &db_map.uid
        );
        Ok(PlaylistDiff::AppendNew(db_map))
    }
}

#[async_trait]
impl LivePlaylist for PlaylistController {
    async fn lock(&self) -> RwLockReadGuard<'_, PlaylistState> {
        self.state.read().await
    }
}
