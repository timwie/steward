use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::config::Config;
use crate::ingame::Server;

/// Use to look up controller, server and mode settings.
#[async_trait]
pub trait LiveSettings: Send + Sync {
    /// Returns `True` if the given login belongs to an admin.
    fn is_admin(&self, login: &str) -> bool;

    /// The time within the outro in which players can vote
    /// for a restart.
    fn vote_duration(&self) -> Duration;

    /// The `.../UserData/Maps` server directory.
    async fn maps_dir(&self) -> PathBuf;
}

#[derive(Clone)]
pub struct SettingsController {
    server: Arc<dyn Server>,
    config: Arc<Config>, // read-only
}

impl SettingsController {
    pub fn init(server: &Arc<dyn Server>, config: Config) -> Self {
        SettingsController {
            server: server.clone(),
            config: Arc::new(config),
        }
    }
}

#[async_trait]
impl LiveSettings for SettingsController {
    fn is_admin(&self, login: &str) -> bool {
        self.config.admin_whitelist.contains(&login.to_string())
    }

    fn vote_duration(&self) -> Duration {
        Duration::from_secs(self.config.vote_duration_secs as u64)
    }

    async fn maps_dir(&self) -> PathBuf {
        self.server.user_data_dir().await.join("Maps")
    }
}
