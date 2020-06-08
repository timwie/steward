use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::config::Config;
use crate::ingame::Server;

/// Use to look up controller, server and mode settings.
#[async_trait]
pub trait LiveSettings: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock_config(&self) -> RwLockReadGuard<'_, Config>;

    /// Returns `True` if the given login belongs to a super admin.
    async fn is_super_admin(&self, login: &str) -> bool {
        let cfg = self.lock_config().await;
        cfg.super_admin_whitelist.contains(&login.to_string())
    }

    /// Returns `True` if the given login belongs to an admin or super admin.
    async fn is_admin(&self, login: &str) -> bool {
        let cfg = self.lock_config().await;
        cfg.admin_whitelist.contains(&login.to_string())
            || cfg.super_admin_whitelist.contains(&login.to_string())
    }

    /// The time within the outro in which players can vote
    /// for a restart.
    async fn vote_duration(&self) -> Duration {
        Duration::from_secs(self.lock_config().await.vote_duration_secs() as u64)
    }

    /// The `.../UserData/Maps` server directory.
    async fn maps_dir(&self) -> PathBuf;
}

#[derive(Clone)]
pub struct SettingsController {
    server: Arc<dyn Server>,
    config: Arc<RwLock<Config>>,
}

impl SettingsController {
    pub fn init(server: &Arc<dyn Server>, config: Config) -> Self {
        SettingsController {
            server: server.clone(),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Edit the controller config, and save the changes in the config file.
    pub async fn edit_config(&self, block: impl Fn(&mut Config)) {
        let mut cfg = self.config.write().await;
        block(&mut *cfg);
        (*cfg).save(); // write to file

        // Sync with server
        let mut mode_options = self.server.mode_options().await;
        mode_options.chat_time_secs = cfg.outro_duration_secs as i32;
        mode_options.time_limit_secs = cfg.race_duration_secs as i32;
        self.server.set_mode_options(&mode_options).await;
    }
}

#[async_trait]
impl LiveSettings for SettingsController {
    async fn lock_config(&self) -> RwLockReadGuard<'_, Config> {
        self.config.read().await
    }

    async fn maps_dir(&self) -> PathBuf {
        self.server.user_data_dir().await.join("Maps")
    }
}
