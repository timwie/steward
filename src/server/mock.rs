use std::path::PathBuf;

use async_trait::async_trait;
use tokio::time::Duration;

use crate::server::*;

pub type Result<T> = std::result::Result<T, Fault>;

#[derive(Clone)]
pub struct Server;

#[async_trait]
impl Calls for Server {
    async fn server_build_info(&self) -> ServerBuildInfo {
        unimplemented!()
    }

    async fn server_net_stats(&self) -> ServerNetStats {
        unimplemented!()
    }

    async fn server_options(&self) -> ServerOptions {
        unimplemented!()
    }

    async fn set_server_options(&self, _options: &ServerOptions) {
        unimplemented!()
    }

    async fn mode(&self) -> ModeInfo {
        unimplemented!()
    }

    async fn set_mode(&self, _script: ModeScript) -> Result<()> {
        unimplemented!()
    }

    async fn mode_options(&self) -> ModeOptions {
        unimplemented!()
    }

    async fn set_mode_options(&self, _options: &ModeOptions) -> Result<()> {
        unimplemented!()
    }

    async fn scores(&self) -> Scores {
        unimplemented!()
    }

    async fn set_player_score(&self, _login: &str, _points: Points) -> Scores {
        unimplemented!()
    }

    async fn set_team_score(&self, _team: TeamId, _points: Points) -> Scores {
        unimplemented!()
    }

    async fn pause_status(&self) -> PauseStatus {
        unimplemented!()
    }

    async fn warmup_status(&self) -> WarmupStatus {
        unimplemented!()
    }

    async fn user_data_dir(&self) -> PathBuf {
        unimplemented!()
    }

    async fn players(&self) -> Vec<PlayerInfo> {
        unimplemented!()
    }

    async fn map(&self, _file_name: &str) -> Result<MapInfo> {
        unimplemented!()
    }

    async fn playlist(&self) -> Vec<PlaylistMap> {
        unimplemented!()
    }

    async fn playlist_current_index(&self) -> Option<usize> {
        unimplemented!()
    }

    async fn playlist_next_index(&self) -> usize {
        unimplemented!()
    }

    async fn playlist_add(&self, _map_file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn playlist_add_all(&self, _map_file_names: Vec<&str>) {
        unimplemented!()
    }

    async fn playlist_remove(&self, _map_file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn playlist_replace(&self, _map_file_names: Vec<&str>) {
        unimplemented!()
    }

    async fn playlist_change_next(&self, _map_index: i32) -> Result<()> {
        unimplemented!()
    }

    async fn load_match_settings(&self, _file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn save_match_settings(&self, _file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn chat_send(&self, _msg: &str) {
        unimplemented!()
    }

    async fn chat_send_to(&self, _msg: &str, _logins: Vec<&str>) -> Result<()> {
        unimplemented!()
    }

    async fn chat_send_from_to(&self, _msg: &str, _from: &str, _logins: Vec<&str>) -> Result<()> {
        unimplemented!()
    }

    async fn send_manialink(&self, _ml: &str) {
        unimplemented!()
    }

    async fn send_manialink_to(&self, _ml: &str, _player_uid: i32) -> Result<()> {
        unimplemented!()
    }

    async fn force_pure_spectator(&self, _player_uid: i32) -> Result<()> {
        unimplemented!()
    }

    async fn blacklist_add(&self, _player_login: &str) -> Result<()> {
        unimplemented!()
    }

    async fn blacklist_remove(&self, _player_login: &str) -> Result<()> {
        unimplemented!()
    }

    async fn blacklist(&self) -> Vec<String> {
        unimplemented!()
    }

    async fn blacklist_load(&self, _file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn blacklist_save(&self, _file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn blacklist_clear(&self, _file_name: &str) -> Result<()> {
        unimplemented!()
    }

    async fn kick_player(&self, _login: &str, _reason: Option<&str>) -> Result<()> {
        unimplemented!()
    }

    async fn shutdown_server(&self) {
        unimplemented!()
    }
}

#[async_trait]
impl SetupCalls for Server {
    async fn authenticate(&self, _username: &str, _password: &str) {
        unimplemented!()
    }

    async fn enable_callbacks(&self) {
        unimplemented!()
    }

    async fn set_api_version(&self) {
        unimplemented!()
    }

    async fn set_checkpoint_event_mode(&self) {
        unimplemented!()
    }

    async fn enable_manual_chat_routing(&self) -> Result<()> {
        unimplemented!()
    }

    async fn clear_manialinks(&self) {
        unimplemented!()
    }
}

#[async_trait]
impl ModeCalls for Server {
    async fn restart_map(&self) {
        unimplemented!()
    }

    async fn end_map(&self) -> Result<()> {
        unimplemented!()
    }
}

#[async_trait]
impl RoundBasedModeCalls for Server {
    async fn pause(&self) -> PauseStatus {
        unimplemented!()
    }

    async fn unpause(&self) -> PauseStatus {
        unimplemented!()
    }

    async fn force_end_warmup(&self) {
        unimplemented!()
    }

    async fn warmup_extend(&self, _duration: Duration) {
        unimplemented!()
    }

    async fn force_end_round(&self) {
        unimplemented!()
    }
}

#[async_trait]
impl ChampionCalls for Server {
    async fn start_new_match(&self) -> Result<()> {
        unimplemented!()
    }

    async fn start_round_nb(&self, _round_nb: i32) -> Result<()> {
        unimplemented!()
    }
}
