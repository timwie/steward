use std::convert::TryFrom;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;
use serde_bytes::ByteBuf;

use async_trait::async_trait;

use crate::api::*;
use crate::xml::*;
use crate::RpcClient;

// Simple macro used to reduce 'Value::from' boilerplate.
macro_rules! args {
    ( $( $args:expr ),* ) => {
        vec![$( Value::from($args) ),*]
    };
}

#[async_trait]
impl Calls for RpcClient {
    async fn authenticate(&self, username: &str, password: &str) {
        self.call_method_unwrap_unit("Authenticate", args!(username, password))
            .await;
    }

    async fn enable_callbacks(&self) {
        self.call_method_unwrap_unit("EnableCallbacks", args!(true))
            .await;
        self.call_script("XmlRpc.EnableCallbacks", args!("true"))
            .await;
    }

    async fn set_api_version(&self) {
        self.call_method_unwrap_unit("SetApiVersion", args!(SERVER_API_VERSION))
            .await;
        self.call_script("XmlRpc.SetApiVersion", args!(SCRIPT_API_VERSION))
            .await;
    }

    async fn enable_manual_chat_routing(&self) {
        self.call_method_unwrap_unit(
            "ChatEnableManualRouting",
            args!(true, true), // enable, but keep auto-forwarding server messages
        )
        .await;
    }

    async fn clear_manialinks(&self) {
        // ignore fault caused by having no players connected
        let _ = self
            .call_method_unit("SendHideManialinkPage", args!())
            .await;
    }

    async fn server_info(&self) -> ServerInfo {
        self.call_method_unwrap("GetVersion", args!()).await
    }

    async fn server_options(&self) -> ServerOptions {
        self.call_method_unwrap("GetServerOptions", args!()).await
    }

    async fn set_server_options(&self, options: &ServerOptions) {
        self.call_method_unwrap_unit("SetServerOptions", args!(to_value(options)))
            .await
    }

    async fn mode(&self) -> ModeInfo {
        self.call_method_unwrap("GetModeScriptInfo", args!()).await
    }

    async fn set_mode(&self, script_text: &str) -> Result<()> {
        self.call_method_unit("SetModeScriptText", args!(escape_xml(script_text)))
            .await
    }

    async fn user_data_dir(&self) -> PathBuf {
        let path_str: String = self.call_method_unwrap("GameDataDirectory", args!()).await;
        Path::new(&path_str)
            .parent()
            .expect("failed to locate server directory")
            .join("UserData")
    }

    async fn mode_options(&self) -> ModeOptions {
        self.call_method_unwrap("GetModeScriptSettings", args!())
            .await
    }

    async fn set_mode_options(&self, options: &ModeOptions) {
        self.call_method_unwrap_unit("SetModeScriptSettings", args!(to_value(options)))
            .await;
    }

    async fn set_ui_properties(&self, xml: &str) {
        self.call_script("Trackmania.UI.SetProperties", args!(escape_xml(xml)))
            .await;
    }

    async fn players(&self) -> Vec<PlayerInfo> {
        self.call_method_unwrap(
            "GetPlayerList",
            args!(-1, 0), // length, offset
        )
        .await
    }

    async fn map(&self, file_name: &str) -> Result<MapInfo> {
        self.call_method("GetMapInfo", args!(file_name)).await
    }

    async fn playlist(&self) -> Vec<MapInfo> {
        self.call_method_unwrap(
            "GetMapList",
            args!(-1, 0), // length, offset
        )
        .await
    }

    async fn playlist_current_index(&self) -> Option<usize> {
        let idx: i32 = self.call_method_unwrap("GetCurrentMapIndex", args!()).await;
        match usize::try_from(idx) {
            Ok(idx) => Some(idx),
            Err(_) => None,
        }
    }

    async fn playlist_add(&self, map_file_name: &str) -> Result<()> {
        self.call_method_unit("AddMap", args!(map_file_name)).await
    }

    async fn playlist_add_all(&self, map_file_names: Vec<&str>) {
        let owned: Vec<Value> = map_file_names
            .iter()
            .map(|f| Value::String((*f).to_string()))
            .collect();
        let _: i32 = self
            .call_method_unwrap("AddMapList", vec![Value::Array(owned)])
            .await;
    }

    async fn playlist_remove(&self, map_file_name: &str) -> Result<()> {
        self.call_method_unit("RemoveMap", args!(map_file_name))
            .await
    }

    async fn playlist_replace(&self, map_file_names: Vec<&str>) {
        let prev_maps: Vec<MapInfo> = self
            .call_method_unwrap(
                "GetMapList",
                args!(-1, 0), // length, offset
            )
            .await;

        let prev_file_names: Vec<Value> = prev_maps
            .iter()
            .map(|info| Value::String(info.file_name.clone()))
            .collect();
        let new_file_names: Vec<Value> = map_file_names
            .iter()
            .map(|f| Value::String((*f).to_string()))
            .collect();
        let _: i32 = self
            .call_method_unwrap("RemoveMapList", vec![Value::Array(prev_file_names)])
            .await;
        let _: i32 = self
            .call_method_unwrap("AddMapList", vec![Value::Array(new_file_names)])
            .await;
    }

    async fn playlist_save(&self, file_name: &str) {
        let _: i32 = self
            .call_method_unwrap("SaveMatchSettings", args!(file_name))
            .await;
    }

    async fn playlist_change_next(&self, map_index: i32) -> Result<()> {
        self.call_method_unit("SetNextMapIndex", args!(map_index))
            .await
    }

    async fn playlist_change_current(&self, map_index: i32) -> Result<()> {
        self.call_method_unit("JumpToMapIndex", args!(map_index))
            .await
    }

    async fn playlist_skip(&self) {
        self.call_method_unwrap_unit("NextMap", args!()).await;
    }

    async fn chat_send(&self, msg: &str) {
        self.call_method_unwrap_unit("ChatSendServerMessage", args!(msg))
            .await;
    }

    async fn chat_send_to(&self, msg: &str, logins: Vec<&str>) -> Result<()> {
        self.call_method_unit("ChatSendServerMessageToLogin", args!(msg, logins.join(",")))
            .await
    }

    async fn chat_send_from_to(&self, msg: &str, from: &str, logins: Vec<&str>) -> Result<()> {
        self.call_method_unit("ChatForwardToLogin", args!(msg, from, logins.join(",")))
            .await
    }

    async fn send_manialink(&self, ml: &str) {
        // 0 = do not auto-hide, false = do not hide on click
        self.call_method_unwrap_unit("SendDisplayManialinkPage", args!(escape_xml(ml), 0, false))
            .await;
    }

    async fn send_manialink_to(&self, ml: &str, player_uid: i32) -> Result<()> {
        // 0 = do not auto-hide, false = do not hide on click
        self.call_method_unit(
            "SendDisplayManialinkPageToId",
            args!(player_uid, escape_xml(ml), 0, false),
        )
        .await
    }

    async fn validation_replay(&self, player_login: &str) -> Result<Vec<u8>> {
        // I cannot igure out how to deserialize into a Vec<u8>,
        // so we'll use "serde_bytes" for that.
        let buf: ByteBuf = self
            .call_method("GetValidationReplay", args!(player_login))
            .await?;
        Ok(buf.into_vec())
    }

    async fn ghost_replay(&self, player_login: &str) -> Result<std::io::Result<Vec<u8>>> {
        // The server will write the ghost replay to disk.
        // We will use a temporary file, read from it, then delete it.

        // To prevent problems that might be caused by calling this function
        // in quick succession, the temporary file should never have the
        // same name twice.
        let unique_file_name = {
            static COUNTER: AtomicUsize = AtomicUsize::new(1);
            format!("tmp_ghost_{}", COUNTER.fetch_add(1, Ordering::Relaxed))
        };

        // .../UserData/Replays/<file_name>.Replay.Gbx
        let replay_path = self
            .user_data_dir()
            .await
            .join("Replays")
            .join(format!("{}.Replay.Gbx", unique_file_name));

        self.call_method_unit(
            "SaveBestGhostsReplay",
            args!(player_login, unique_file_name),
        )
        .await?;

        Ok(consume_file(&replay_path))
    }

    async fn force_pure_spectator(&self, player_uid: i32) -> Result<()> {
        // This value is documented as "spectator but keep selectable",
        // which probably means that you can switch back to a playing slot.
        const SPECTATOR_MODE: i32 = 3;

        self.call_method_unit("ForceSpectatorId", args!(player_uid, SPECTATOR_MODE))
            .await?;
        self.call_method_unit("SpectatorReleasePlayerSlotId", args!(player_uid))
            .await
    }

    async fn request_scores(&self) {
        let response_id = gen_response_id();
        let arg_list = Value::Array(vec![Value::String(response_id.clone())]);
        let call = Call {
            name: "TriggerModeScriptEventArray".to_string(),
            args: args!("Trackmania.GetScores", arg_list),
        };
        self.trigger_callback(response_id, call).await;
    }

    async fn blacklist_add(&self, player_login: &str) -> Result<()> {
        self.call_method_unit("BlackList", args!(player_login))
            .await
    }

    async fn blacklist_remove(&self, player_login: &str) -> Result<()> {
        self.call_method_unit("UnBlackList", args!(player_login))
            .await
    }

    async fn blacklist(&self) -> Vec<String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct BlacklistPlayer {
            pub login: String,
        }

        let players: Vec<BlacklistPlayer> = self
            .call_method_unwrap(
                "GetBlackList",
                args!(-1, 0), // length, offset
            )
            .await;

        players.into_iter().map(|p| p.login).collect()
    }

    async fn load_blacklist(&self, file_name: &str) -> Result<()> {
        self.call_method_unit("LoadBlackList", args!(file_name))
            .await
    }

    async fn save_blacklist(&self, file_name: &str) -> Result<()> {
        self.call_method_unit("SaveBlackList", args!(file_name))
            .await
    }

    async fn kick_player(&self, login: &str, reason: Option<&str>) -> Result<()> {
        let args = match reason {
            Some(reason) => args!(login, reason),
            None => args!(login),
        };
        self.call_method_unit("Kick", args).await
    }

    async fn net_stats(&self) -> NetStats {
        self.call_method_unwrap("GetNetworkStats", args!()).await
    }

    async fn stop_server(&self) {
        self.call_method_unwrap_unit("StopServer", args!()).await;
        self.call_method_unwrap_unit("QuitGame", args!()).await;
    }
}

impl RpcClient {
    /// Call an XML-RPC method, and handle faults.
    async fn call_method<T>(&self, method_name: &str, args: Vec<Value>) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.call(Call {
            args,
            name: String::from(method_name),
        })
        .await
    }

    /// Call an XML-RPC method that does not return a result, and handle faults.
    async fn call_method_unit(&self, method_name: &str, args: Vec<Value>) -> Result<()> {
        self.call_method::<bool>(method_name, args)
            .await
            .and_then(|_| Ok(()))
    }

    /// Call an XML-RPC method, and do not expect any faults.
    /// This will panic if a fault is encountered.
    async fn call_method_unwrap<T>(&self, method_name: &str, args: Vec<Value>) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        self.call_unwrap(Call {
            args,
            name: String::from(method_name),
        })
        .await
    }

    /// Call an XML-RPC method that does not return a result,
    /// and do not expect any faults.
    /// This will panic if a fault is encountered.
    async fn call_method_unwrap_unit(&self, method_name: &str, args: Vec<Value>) {
        assert!(
            self.call_unwrap::<bool>(Call {
                args,
                name: String::from(method_name),
            })
            .await,
            "expected method to return 'true'"
        );
    }

    /// Call a mode script XML-RPC method.
    /// Script methods that return an answer will send it using a script callback.
    async fn call_script(&self, method_name: &str, args: Vec<Value>) {
        self.call_method_unwrap_unit("TriggerModeScriptEventArray", args!(method_name, args))
            .await;
    }
}

/// Generate a unique `response_id` for triggering callbacks.
fn gen_response_id() -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

/// Escape an XML string so that it can be included
/// in an XML-RPC call.
fn escape_xml(input: &str) -> String {
    // 'with_capacity' is only the initial buffer size, not a hard limit
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\'' => result.push_str("&apos;"),
            '"' => result.push_str("&quot;"),
            o => result.push(o),
        }
    }
    result
}

/// Read a file into memory, and delete it.
fn consume_file(file_path: &PathBuf) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let metadata = fs::metadata(file_path)?;
    let mut buffer = vec![0; metadata.len() as usize];
    f.read_exact(&mut buffer)?;
    fs::remove_file(file_path)?;
    Ok(buffer)
}
