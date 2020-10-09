use async_trait::async_trait;

use crate::database::Result;
use crate::server::{DisplayString, PlayerInfo};

/// Database player that has joined the server at least once.
#[derive(Debug, PartialEq, Eq)]
pub struct Player {
    /// Player login.
    pub login: String,

    /// Formatted display name.
    pub display_name: DisplayString,
}

#[async_trait]
pub trait PlayerQueries {
    /// Return the specified player, or `None` if no such player exists in the database.
    async fn player(&self, login: &str) -> Result<Option<Player>>;

    /// Return players for every input login that exists in the database.
    async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>>;

    /// Insert a player into the database.
    /// Update their display name if the player already exists.
    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()>;

    /// Delete a player, their preferences, and their records.
    /// The data is lost forever.
    async fn delete_player(&self, player_login: &str) -> Result<Option<Player>>;
}
