use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde::Deserialize;
use tokio::time::Duration;

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

    async fn set_mode(&self, script: ModeScript) -> Result<()> {
        self.call_method_unit("SetScriptName", args!(script.file_name()))
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
        let mode_info = self.mode().await;

        macro_rules! get {
            ($typ:ty) => {
                self.call_method_unwrap::<$typ>("GetModeScriptSettings", args!())
                    .await
            };
        }

        match mode_info.script {
            ModeScript::TimeAttack => ModeOptions::TimeAttack(get!(TimeAttackOptions)),
            _ => panic!("modes other than TimeAttack are not supported"),
        }
    }

    async fn set_mode_options(&self, options: &ModeOptions) -> Result<()> {
        let options = match options {
            ModeOptions::TimeAttack(options) => to_value(options),
            _ => panic!("modes other than TimeAttack are not supported"),
        };

        self.call_method_unit("SetModeScriptSettings", args!(options))
            .await
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

    async fn playlist(&self) -> Vec<PlaylistMap> {
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

    async fn playlist_next_index(&self) -> usize {
        let idx: i32 = self.call_method_unwrap("GetNextMapIndex", args!()).await;
        idx as usize
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
        let prev_maps: Vec<PlaylistMap> = self
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

    async fn restart_map(&self) {
        self.call_method_unwrap_unit("RestartMap", args!()).await;
    }

    async fn end_map(&self) {
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

    async fn force_pure_spectator(&self, player_uid: i32) -> Result<()> {
        // This value is documented as "spectator but keep selectable",
        // which probably means that you can switch back to a playing slot.
        const SPECTATOR_MODE: i32 = 3;

        self.call_method_unit("ForceSpectatorId", args!(player_uid, SPECTATOR_MODE))
            .await?;
        self.call_method_unit("SpectatorReleasePlayerSlotId", args!(player_uid))
            .await
    }

    async fn scores(&self) -> Scores {
        let cb = self
            .call_script_result("Trackmania.GetScores", args!())
            .await;

        if let Callback::Scores { scores } = cb {
            return scores;
        }
        panic!("unexpected callback {:?}", cb);
    }

    async fn pause_status(&self) -> WarmupOrPauseStatus {
        let cb = self
            .call_script_result("Maniaplanet.Pause.GetStatus", args!())
            .await;

        if let Callback::PauseStatus(status) = cb {
            return status;
        }
        panic!("unexpected callback {:?}", cb);
    }

    async fn warmup_status(&self) -> WarmupOrPauseStatus {
        let cb = self
            .call_script_result("Trackmania.WarmUp.GetStatus", args!())
            .await;

        if let Callback::WarmupStatus(status) = cb {
            return status;
        }
        panic!("unexpected callback {:?}", cb);
    }

    async fn pause(&self) -> WarmupOrPauseStatus {
        let cb = self
            .call_script_result("Maniaplanet.Pause.SetActive", args!("true"))
            .await;

        if let Callback::PauseStatus(status) = cb {
            return status;
        }
        panic!("unexpected callback {:?}", cb);
    }

    async fn unpause(&self) -> WarmupOrPauseStatus {
        let cb = self
            .call_script_result("Maniaplanet.Pause.SetActive", args!("false"))
            .await;

        if let Callback::PauseStatus(status) = cb {
            return status;
        }
        panic!("unexpected callback {:?}", cb);
    }

    async fn force_end_warmup(&self) {
        self.call_script("Trackmania.WarmUp.ForceStop", args!())
            .await;
    }

    async fn warmup_extend(&self, duration: Duration) {
        let millis = duration.as_millis().to_string();
        self.call_script("Maniaplanet.WarmUp.Extend", args!(millis))
            .await;
    }

    async fn force_end_round(&self) {
        self.call_script("Trackmania.ForceEndRound", args!()).await;
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
            .map(|_| ())
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
    ///
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

    /// Call a mode script XML-RPC method that returns a result through a callback.
    async fn call_script_result(&self, method_name: &str, mut args: Vec<Value>) -> Callback {
        let response_id = gen_response_id();
        args.push(Value::String(response_id.clone()));
        let args = Value::Array(args);

        let call = Call {
            name: "TriggerModeScriptEventArray".to_string(),
            args: args!(method_name, args),
        };

        self.trigger_callback(response_id, call).await
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
