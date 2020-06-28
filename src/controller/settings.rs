use std::path::PathBuf;
use std::sync::Arc;

use chrono::Duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::config::Config;
use crate::event::ConfigDiff;
use crate::server::Server;

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

    /// The time within the outro in which players can vote for a restart.
    async fn vote_duration(&self) -> Duration {
        Duration::seconds(self.lock_config().await.vote_duration_secs() as i64)
    }

    /// The duration of the outro at the end of a map.
    async fn outro_duration(&self) -> Duration {
        Duration::seconds(self.lock_config().await.outro_duration_secs as i64)
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
    pub async fn init(server: &Arc<dyn Server>, config: Config) -> Self {
        set_mode_options(server, &config).await;
        SettingsController {
            server: server.clone(),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Update the public parts of the controller config.
    pub async fn set_public_config(&self, new_cfg: PublicConfig) -> Vec<ConfigDiff> {
        use ConfigDiff::*;

        let mut diffs = Vec::new();
        let mut cfg = self.config.write().await;

        if cfg.outro_duration_secs != new_cfg.outro_duration_secs {
            diffs.push(NewOutroDuration {
                secs: new_cfg.outro_duration_secs,
            });

            cfg.outro_duration_secs = new_cfg.outro_duration_secs;
            set_mode_options(&self.server, &cfg).await;
        }

        if cfg.timelimit_factor != new_cfg.timelimit_factor
            || cfg.timelimit_max_secs != new_cfg.timelimit_max_secs
            || cfg.timelimit_min_secs != new_cfg.timelimit_min_secs
        {
            diffs.push(NewTimeLimit {
                timelimit_factor: new_cfg.timelimit_factor,
                timelimit_max_secs: new_cfg.timelimit_max_secs,
                timelimit_min_secs: new_cfg.timelimit_min_secs,
            });

            cfg.timelimit_factor = new_cfg.timelimit_factor;
            cfg.timelimit_max_secs = new_cfg.timelimit_max_secs;
            cfg.timelimit_min_secs = new_cfg.timelimit_min_secs;
        }

        if !diffs.is_empty() {
            (*cfg).save(); // write to file
        }

        diffs
    }

    /// Returns a public subset of the controller config, omitting credentials etc.
    pub async fn public_config(&self) -> PublicConfig {
        let cfg = self.config.read().await;
        PublicConfig {
            timelimit_factor: cfg.timelimit_factor,
            timelimit_max_secs: cfg.timelimit_max_secs,
            timelimit_min_secs: cfg.timelimit_min_secs,
            outro_duration_secs: cfg.outro_duration_secs,
        }
    }
}

async fn set_mode_options(server: &Arc<dyn Server>, config: &Config) {
    let mut mode_options = server.mode_options().await;
    mode_options.chat_time_secs = config.outro_duration_secs as i32;
    server.set_mode_options(&mode_options).await;
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

/// A public subset of the controller config, omitting credentials etc,
/// that is ready to be displayed and edited in-game.
#[derive(Deserialize, Serialize)]
pub struct PublicConfig {
    pub timelimit_factor: u32,
    pub timelimit_max_secs: u32,
    pub timelimit_min_secs: u32,
    pub outro_duration_secs: u32,
}

impl PublicConfig {
    pub fn write(&self) -> String {
        toml::to_string(&self).expect("failed to serialize ingame config")
    }

    pub fn read(serialized: &str) -> Result<PublicConfig, PublicConfigError> {
        use PublicConfigError::*;

        let cfg: PublicConfig = toml::from_str(serialized)?;

        if cfg.timelimit_factor == 0 {
            return Err(TimeLimitFactorCannotBeZero);
        }
        if cfg.timelimit_max_secs == 0 {
            return Err(TimeLimitMaxCannotBeZero);
        }
        if cfg.timelimit_min_secs >= cfg.timelimit_max_secs {
            return Err(TimeLimitMinGreaterThanMax);
        }

        Ok(cfg)
    }
}

/// Failed checks when editing the public config.
#[derive(Error, Debug)]
pub enum PublicConfigError {
    #[error("Not a valid config")]
    ParseError(#[from] toml::de::Error),

    #[error("timelimit_factor must be greater than zero")]
    TimeLimitFactorCannotBeZero,

    #[error("timelimit_max_secs must be greater than zero")]
    TimeLimitMaxCannotBeZero,

    #[error("timelimit_max_secs must be greater than timelimit_min_secs")]
    TimeLimitMinGreaterThanMax,
}
