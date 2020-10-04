use async_trait::async_trait;
use tokio_postgres::Row;

use crate::database::api::{Player, PlayerQueries};
use crate::database::{DatabaseClient, Result};
use crate::server::{DisplayString, PlayerInfo};

#[async_trait]
impl PlayerQueries for DatabaseClient {
    async fn player(&self, login: &str) -> Result<Option<Player>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = $1
        "#;
        let row = conn.query_opt(stmt, &[&login]).await?;
        Ok(row.map(Player::from))
    }

    async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = ANY($1::text[])
        "#;
        let rows = conn.query(stmt, &[&logins]).await?;
        Ok(rows.into_iter().map(Player::from).collect())
    }

    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            INSERT INTO steward.player
                (login, display_name)
            VALUES
                ($1, $2)
            ON CONFLICT (login)
            DO UPDATE SET
                display_name = excluded.display_name
        "#;
        let _ = conn
            .execute(
                stmt,
                &[&player.login, &player.display_name.formatted.trim()],
            )
            .await?;
        Ok(())
    }

    async fn delete_player(&self, player_login: &str) -> Result<Option<Player>> {
        let mut conn = self.pool.get().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.ta_preference WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.record WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.player WHERE login = $1 RETURNING *";
        let maybe_row = transaction.query_opt(stmt, &[&player_login]).await?;
        let maybe_player = maybe_row.map(Player::from);

        transaction.commit().await?;
        Ok(maybe_player)
    }
}

impl From<Row> for Player {
    fn from(row: Row) -> Self {
        Player {
            login: row.get("login"),
            display_name: DisplayString::from(row.get("display_name")),
        }
    }
}
