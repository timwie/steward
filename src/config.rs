use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::constants::{CONFIG_ENV_VAR, VOTE_DURATION_RATIO};

/// Controller config.
#[derive(Deserialize, Serialize)]
pub struct Config {
    /// The address of the game server's XML-RPC port, f.e. "127.0.0.1:5000".
    ///
    /// A game server will listen on the port 5000 by default, where each
    /// additional instance will use 5001, 5002, etc. A game client
    /// will also reserve a port, which is relevant for development:
    /// if you start the game first, the server will listen at port 5001.
    ///
    /// It is also possible to select a specific port, using the `<xmlrpc_port>`
    /// setting in the server config.
    pub rpc_address: String,

    /// The "SuperAdmin" login defined in the `<authorization_levels>`
    /// server config in `/UserData/Config/*.txt`.
    pub rpc_login: String,

    /// The "SuperAdmin" password defined in the `<authorization_levels>`
    /// server config in `/UserData/Config/*.txt`.
    pub rpc_password: String,

    /// Connection configuration parsed from libpq-style connection strings, f.e.
    /// `host=127.0.0.1 port=5432 user=postgres password=123 connect_timeout=10`.
    ///
    /// Reference: https://www.postgresql.org/docs/9.3/libpq-connect.html#LIBPQ-CONNSTRING
    pub postgres_connection: String,

    /// List of player logins that can execute super admin commands.
    pub super_admin_whitelist: Vec<String>,

    /// List of player logins that can execute admin commands.
    pub admin_whitelist: Vec<String>,

    /// To calculate the time limit of a map, this factor is applied to either the
    /// author time or the top record.
    pub time_limit_factor: u32,

    /// The maximum time limit in seconds.
    pub time_limit_max_secs: u32,

    /// The minimum time limit in seconds.
    pub time_limit_min_secs: u32,

    /// The time spent on a map after the race ends in seconds.
    /// Overrides the `S_ChatTime` mode setting.
    ///
    /// This should be long enough to allow for widget interaction
    /// after a race.
    ///
    /// Votes during the outro will be open for two thirds of this value.
    pub outro_duration_secs: u32,
}

impl Config {
    /// The time during which players can still vote for a restart
    /// after a race ends. The next map will be decided after
    /// this duration.
    pub fn vote_duration_secs(&self) -> u32 {
        (self.outro_duration_secs as f32 * VOTE_DURATION_RATIO) as u32
    }

    /// Read the config file listed in the `STEWARD_CONFIG` environment variable.
    ///
    /// # Panics
    /// - when `STEWARD_CONFIG` is not set
    /// - when `STEWARD_CONFIG` does not point to a valid TOML config
    /// - when the file cannot be parsed
    pub fn load() -> Config {
        let f = Self::path().unwrap_or_else(|| {
            panic!("cannot locate config: use the '{}' env var", CONFIG_ENV_VAR)
        });
        let f_str = std::fs::read_to_string(f).expect("failed to read config file");
        let cfg: Config = toml::from_str(&f_str).expect("failed to parse config file");
        cfg
    }

    /// Overwrite the config file listed in the `STEWARD_CONFIG` environment variable.
    ///
    /// # Panics
    /// - when `STEWARD_CONFIG` is not set
    /// - when the file cannot be overwritten
    pub fn save(&self) {
        let mut config_str = toml::to_string(&self).expect("failed to compose config file");

        // Since all comments are removed from a previous config file,
        // we can at least add a link to the default config.
        let reference_link =
            "# Reference: https://github.com/timwie/steward/blob/master/config/steward.toml\n";
        config_str.insert_str(0, reference_link);

        let f = Self::path().unwrap_or_else(|| {
            panic!("cannot locate config: use the '{}' env var", CONFIG_ENV_VAR)
        });
        std::fs::write(f, config_str).expect("failed to overwrite config file");
    }

    fn path() -> Option<PathBuf> {
        match std::env::var(CONFIG_ENV_VAR) {
            Ok(f) => Some(PathBuf::from(f)).filter(|p| p.is_file()),
            Err(_) => None,
        }
    }
}

/// A public subset of the controller config, omitting credentials etc,
/// that is ready to be displayed and edited in-game.
#[derive(Deserialize, Serialize)]
pub struct PublicConfig {
    pub time_limit_factor: u32,
    pub time_limit_max_secs: u32,
    pub time_limit_min_secs: u32,
    pub outro_duration_secs: u32,
}

impl Config {
    pub fn public(&self) -> PublicConfig {
        PublicConfig {
            time_limit_factor: self.time_limit_factor,
            time_limit_max_secs: self.time_limit_max_secs,
            time_limit_min_secs: self.time_limit_min_secs,
            outro_duration_secs: self.outro_duration_secs,
        }
    }
}

impl PublicConfig {
    pub fn write(&self) -> String {
        toml::to_string(&self).expect("failed to serialize ingame config")
    }

    pub fn read(serialized: &str) -> Result<PublicConfig, PublicConfigError> {
        use PublicConfigError::*;

        let cfg: PublicConfig = toml::from_str(serialized)?;

        if cfg.time_limit_factor == 0 {
            return Err(TimeLimitFactorCannotBeZero);
        }
        if cfg.time_limit_max_secs == 0 {
            return Err(TimeLimitMaxCannotBeZero);
        }
        if cfg.time_limit_min_secs >= cfg.time_limit_max_secs {
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

    #[error("time_limit_factor must be greater than zero")]
    TimeLimitFactorCannotBeZero,

    #[error("time_limit_max_secs must be greater than zero")]
    TimeLimitMaxCannotBeZero,

    #[error("time_limit_max_secs must be greater than time_limit_min_secs")]
    TimeLimitMinGreaterThanMax,
}
