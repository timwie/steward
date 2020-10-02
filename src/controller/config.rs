use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::config::{Config, TimeAttackConfig};
use crate::event::ConfigDiff;
use crate::server::{Calls, ModeOptions, Server};

/// Use to look up controller and server configs.
#[async_trait]
pub trait LiveConfig: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, Config>;

    /// Returns `True` if the given login belongs to a super admin.
    async fn is_super_admin(&self, login: &str) -> bool {
        let config = self.lock().await;
        config.super_admin_whitelist.contains(&login.to_string())
    }

    /// Returns `True` if the given login belongs to an admin or super admin.
    async fn is_admin(&self, login: &str) -> bool {
        let config = self.lock().await;
        config.admin_whitelist.contains(&login.to_string())
            || config.super_admin_whitelist.contains(&login.to_string())
    }

    /// The time within the outro in which players can vote for a restart.
    async fn vote_duration(&self) -> Duration {
        Duration::seconds(self.lock().await.timeattack.vote_duration_secs() as i64)
    }

    /// The duration of the outro at the end of a map.
    async fn outro_duration(&self) -> Duration {
        Duration::seconds(self.lock().await.timeattack.outro_duration_secs as i64)
    }

    /// The `.../UserData/Maps` server directory.
    async fn maps_dir(&self) -> PathBuf;
}

#[derive(Clone)]
pub struct ConfigController {
    state: Arc<RwLock<Config>>,
    server: Server,
}

impl ConfigController {
    pub async fn init(server: &Server, config: Config) -> Self {
        set_mode_options(server, &config).await;
        ConfigController {
            state: Arc::new(RwLock::new(config)),
            server: server.clone(),
        }
    }

    /// Update the public parts of the controller config.
    pub async fn set_mode_config(&self, new_cfg: TimeAttackConfig) -> Vec<ConfigDiff> {
        use ConfigDiff::*;

        let mut diffs = Vec::new();
        let mut cfg = self.state.write().await;

        if cfg.timeattack.outro_duration_secs != new_cfg.outro_duration_secs {
            diffs.push(NewOutroDuration {
                secs: new_cfg.outro_duration_secs,
            });

            cfg.timeattack.outro_duration_secs = new_cfg.outro_duration_secs;
            set_mode_options(&self.server, &cfg).await;
        }

        if cfg.timeattack.time_limit_factor != new_cfg.time_limit_factor
            || cfg.timeattack.time_limit_max_secs != new_cfg.time_limit_max_secs
            || cfg.timeattack.time_limit_min_secs != new_cfg.time_limit_min_secs
        {
            diffs.push(NewTimeLimit {
                time_limit_factor: new_cfg.time_limit_factor,
                time_limit_max_secs: new_cfg.time_limit_max_secs,
                time_limit_min_secs: new_cfg.time_limit_min_secs,
            });

            cfg.timeattack.time_limit_factor = new_cfg.time_limit_factor;
            cfg.timeattack.time_limit_max_secs = new_cfg.time_limit_max_secs;
            cfg.timeattack.time_limit_min_secs = new_cfg.time_limit_min_secs;
        }

        if !diffs.is_empty() {
            (*cfg).save(); // write to file
        }

        diffs
    }

    pub async fn mode_config(&self) -> TimeAttackConfig {
        let config = self.state.read().await;
        config.timeattack
    }
}

async fn set_mode_options(server: &Server, config: &Config) {
    let mode_options = server.mode_options().await;
    if let ModeOptions::TimeAttack(mut options) = mode_options {
        options.chat_time_secs = config.timeattack.outro_duration_secs as i32;
        server
            .set_mode_options(&ModeOptions::TimeAttack(options))
            .await
            .expect("failed to set mode options");
    }
}

#[async_trait]
impl LiveConfig for ConfigController {
    async fn lock(&self) -> RwLockReadGuard<'_, Config> {
        self.state.read().await
    }

    async fn maps_dir(&self) -> PathBuf {
        self.server.user_data_dir().await.join("Maps")
    }
}
