use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use include_dir::{include_dir, Dir};
use tokio_postgres::Row;

use crate::database::queries::Queries;
use crate::database::structs::*;
use crate::server::{DisplayString, PlayerInfo};

/// Connect to the Postgres database and open a connection pool.
pub async fn db_connect(conn: &str, timeout: Duration) -> Option<Arc<dyn Queries>> {
    pg_connect(conn, timeout)
        .await
        .map(|client| Arc::new(client) as Arc<dyn Queries>)
}

pub async fn pg_connect(conn: &str, timeout: Duration) -> Option<PostgresClient> {
    let config = tokio_postgres::config::Config::from_str(&conn)
        .expect("failed to parse postgres connection string");

    let pg_mgr = bb8_postgres::PostgresConnectionManager::new(config, tokio_postgres::NoTls);

    let pool = bb8::Pool::builder()
        .build(pg_mgr)
        .await
        .expect("failed to build database pool");

    let connect_or_timeout = tokio::time::timeout(timeout, pool.get());

    match connect_or_timeout.await {
        Ok(conn) => {
            conn.expect("failed to connect to database");
        }
        Err(_) => return None,
    }

    Some(PostgresClient(pool))
}

/// A connection pool that maintains a set of open
/// connections to the database, handing them out for
/// repeated use.
type PostgresPool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

/// `Queries` implementation that produces `bb8::RunError<tokio_postgres::Error>s`.
#[derive(Clone)]
pub struct PostgresClient(pub PostgresPool);

impl PostgresClient {
    async fn playlist_edit(&self, map_uid: &str, in_playlist: bool) -> Result<Option<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            UPDATE steward.map
            SET in_playlist = $2
            WHERE uid = $1
            RETURNING uid, file_name, name, author_login, author_display_name, added_since, in_playlist, exchange_id
        "#;
        let row = conn.query_opt(stmt, &[&map_uid, &in_playlist]).await?;
        Ok(row.map(Map::from))
    }
}

#[async_trait]
impl Queries for PostgresClient {
    async fn migrate(&self) -> Result<()> {
        // Include all migration statements at compile-time:
        static MIGRATION_DIR: Dir = include_dir!("src/res/migrations/");

        let stmts = |nb: usize| {
            MIGRATION_DIR
                .get_file(format!("{}.sql", nb))
                .and_then(|f| f.contents_utf8())
                .unwrap_or_else(|| panic!("failed to find statements for migration {}", nb))
        };

        let mut conn = self.0.get().await?;
        let transaction = conn.transaction().await?;

        // Run the initial 'migration' that only creates the metadata
        // table if it doesn't exist.
        transaction.batch_execute(stmts(0)).await?;

        // Get the most recently executed migration number.
        let at_migration: usize = {
            let stmt = "SELECT at_migration FROM steward.meta";
            let row = transaction.query_one(stmt, &[]).await?;
            row.get::<usize, i32>(0) as usize
        };
        log::debug!("database at migration {}", at_migration);

        let most_recent_migration: usize = MIGRATION_DIR.files().len() - 1;
        let pending_migrations = at_migration + 1..most_recent_migration + 1;
        for i in pending_migrations {
            log::info!("run database migration {}...", i);
            transaction.batch_execute(stmts(i)).await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    async fn player(&self, login: &str) -> Result<Option<Player>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = $1
        "#;
        let row = conn.query_opt(stmt, &[&login]).await?;
        Ok(row.map(Player::from))
    }

    async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = ANY($1::text[])
        "#;
        let rows = conn.query(stmt, &[&logins]).await?;
        Ok(rows.into_iter().map(Player::from).collect())
    }

    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()> {
        let conn = self.0.get().await?;
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

    async fn add_history(
        &self,
        player_login: &str,
        map_uid: &str,
        last_played: &NaiveDateTime,
    ) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.history
                (player_login, map_uid, last_played)
            VALUES
                ($1, $2, $3)
            ON CONFLICT (player_login, map_uid)
            DO UPDATE SET
                last_played = excluded.last_played
        "#;
        let _ = conn
            .execute(stmt, &[&player_login, &map_uid, &last_played])
            .await?;
        Ok(())
    }

    async fn history(&self, player_login: &str) -> Result<Vec<History>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT
                m.uid map_uid,
                h.last_played,
                RANK () OVER (
                    ORDER BY h.last_played DESC NULLS LAST
                ) - 1 nb_maps_since
            FROM steward.map m
            LEFT JOIN steward.history h ON m.uid = h.map_uid
            WHERE h.player_login is NULL OR h.player_login = $1
        "#;
        let rows = conn.query(stmt, &[&player_login]).await?;
        let result = rows
            .into_iter()
            .map(|row| History {
                player_login: player_login.to_string(),
                map_uid: row.get("map_uid"),
                last_played: row.get("last_played"),
                nb_maps_since: usize::try_from(row.get::<_, i64>("nb_maps_since"))
                    .expect("failed to convert nb_maps_since"),
            })
            .collect();
        Ok(result)
    }

    async fn map_files(&self) -> Result<Vec<MapEvidence>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(MapEvidence::from).collect();
        Ok(maps)
    }

    async fn maps(&self) -> Result<Vec<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT uid, file_name, name, author_login, author_display_name, author_millis, added_since, in_playlist, exchange_id
            FROM steward.map
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    async fn playlist(&self) -> Result<Vec<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT uid, file_name, name, author_login, author_display_name, author_millis, added_since, in_playlist, exchange_id
            FROM steward.map
            WHERE in_playlist
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    async fn map(&self, map_uid: &str) -> Result<Option<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT uid, file_name, name, author_login, author_display_name, author_millis, added_since, in_playlist, exchange_id
            FROM steward.map
            WHERE uid = $1
        "#;
        let row = conn.query_opt(stmt, &[&map_uid]).await?;
        Ok(row.map(Map::from))
    }

    async fn upsert_map(&self, map: &MapEvidence) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.map
                (uid, file_name, file,
                 name, author_login, author_display_name,
                 author_millis, added_since, in_playlist,
                 exchange_id)
            VALUES
                ($1, $2, $3,
                 $4, $5, $6,
                 $7, $8, $9,
                 $10)
            ON CONFLICT (uid)
            DO UPDATE SET
                file_name = excluded.file_name,
                exchange_id = COALESCE(excluded.exchange_id, steward.map.exchange_id)
        "#;
        let _ = conn
            .execute(
                stmt,
                &[
                    &map.metadata.uid,
                    &map.metadata.file_name,
                    &map.data,
                    &map.metadata.name.formatted.trim(),
                    &map.metadata.author_login,
                    &map.metadata.author_display_name.formatted.trim(),
                    &map.metadata.author_millis,
                    &map.metadata.added_since,
                    &map.metadata.in_playlist,
                    &map.metadata.exchange_id,
                ],
            )
            .await?;
        Ok(())
    }

    async fn playlist_add(&self, map_uid: &str) -> Result<Option<Map>> {
        self.playlist_edit(map_uid, true).await
    }

    async fn playlist_remove(&self, map_uid: &str) -> Result<Option<Map>> {
        self.playlist_edit(map_uid, false).await
    }

    async fn nb_records(&self, map_uid: &str) -> Result<i64> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(*) AS INTEGER
            FROM steward.record
            WHERE map_uid = $1;
        "#;
        let row = conn.query_one(stmt, &[&map_uid]).await?;
        Ok(row.get(0))
    }

    async fn records(
        &self,
        map_uids: Vec<&str>,
        player_logins: Vec<&str>,
        limit_per_map: Option<i64>,
    ) -> Result<Vec<Record>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT
                r.map_uid, r.pos, r.millis, r.timestamp,
                p.login, p.display_name,
                s.cp_millis, s.cp_speed
            FROM (
                SELECT
                   map_uid,
                   player_login,
                   millis,
                   timestamp,
                   RANK () OVER (
                      ORDER BY millis ASC
                   ) pos
                FROM steward.record
                WHERE CARDINALITY($1::text[]) = 0 OR map_uid = ANY($1::text[])
                LIMIT $3
            ) r
            NATURAL JOIN (
                SELECT
                    map_uid,
                    player_login,
                    ARRAY_AGG(cp_millis ORDER BY index ASC) cp_millis,
                    ARRAY_AGG(cp_speed ORDER BY index ASC) cp_speed
                FROM steward.sector
                WHERE CARDINALITY($2::text[]) = 0 OR player_login = ANY($2::text[])
                GROUP BY map_uid, player_login
            ) s
            INNER JOIN steward.player p ON r.player_login = p.login
        "#;
        let rows = conn
            .query(stmt, &[&map_uids, &player_logins, &limit_per_map])
            .await?;
        let records = rows
            .into_iter()
            .map(|row| {
                let cp_millis: Vec<i32> = row.get("cp_millis");
                let cp_speed: Vec<f32> = row.get("cp_speed");
                let sectors = cp_millis
                    .into_iter()
                    .zip(cp_speed.into_iter())
                    .enumerate()
                    .map(|(idx, (millis, speed))| RecordSector {
                        index: idx as i32,
                        cp_millis: millis,
                        cp_speed: speed,
                    })
                    .collect();
                Record {
                    map_uid: row.get("map_uid"),
                    map_rank: row.get("pos"),
                    player_login: row.get("login"),
                    player_display_name: DisplayString::from(row.get("display_name")),
                    timestamp: row.get("timestamp"),
                    millis: row.get("millis"),
                    sectors,
                }
            })
            .collect();
        Ok(records)
    }

    async fn nb_players_with_record(&self) -> Result<i64> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(DISTINCT player_login)
            FROM steward.record
        "#;
        let row = conn.query_one(stmt, &[]).await?;
        Ok(row.get(0))
    }

    async fn maps_without_player_record(&self, player_login: &str) -> Result<Vec<String>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT m.uid
            FROM steward.map m
            LEFT JOIN (
                SELECT map_uid FROM steward.record WHERE player_login = $1
            ) r
            ON m.uid = r.map_uid
            WHERE r.map_uid IS NULL
        "#;
        let rows = conn.query(stmt, &[&player_login]).await?;
        let maps = rows.iter().map(|row| row.get(0)).collect();
        Ok(maps)
    }

    async fn record_preview(&self, record: &RecordEvidence) -> Result<i64> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(*)
            FROM steward.record r
            WHERE map_uid = $1 AND r.player_login != $2 AND r.millis < $3
        "#;
        let row = conn
            .query_one(
                stmt,
                &[&record.map_uid, &record.player_login, &record.millis],
            )
            .await?;
        Ok(1 + row.get::<usize, i64>(0))
    }

    async fn upsert_record(&self, rec: &RecordEvidence) -> Result<()> {
        assert_eq!(
            rec.millis,
            rec.sectors.last().expect("empty sectors").cp_millis,
            "inconsistency: run's total millis != last cp millis"
        );

        let mut conn = self.0.get().await?;

        let transaction = conn.transaction().await?;

        let insert_record_stmt = r#"
            INSERT INTO steward.record
                (player_login, map_uid, millis, timestamp)
            VALUES
                ($1, $2, $3, $4)
            ON CONFLICT (player_login, map_uid)
            DO UPDATE SET
                millis = excluded.millis,
                timestamp = excluded.timestamp
        "#;

        let _ = transaction
            .execute(
                insert_record_stmt,
                &[&rec.player_login, &rec.map_uid, &rec.millis, &rec.timestamp],
            )
            .await?;

        let insert_sector_stmt = r#"
            INSERT INTO steward.sector
                (player_login, map_uid, index, cp_millis, cp_speed)
            VALUES
                ($1, $2, $3, $4, $5)
            ON CONFLICT (player_login, map_uid, index)
            DO UPDATE SET
                cp_millis = excluded.cp_millis,
                cp_speed = excluded.cp_speed
        "#;

        for sector in &rec.sectors {
            let _ = transaction
                .execute(
                    insert_sector_stmt,
                    &[
                        &rec.player_login,
                        &rec.map_uid,
                        &sector.index,
                        &sector.cp_millis,
                        &sector.cp_speed,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT * FROM steward.preference
            WHERE player_login = $1 AND value IS NOT NULL
        "#;
        let rows = conn.query(stmt, &[&player_login]).await?;
        let prefs = rows.into_iter().map(Preference::from).collect();
        Ok(prefs)
    }

    async fn count_map_preferences(&self, map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT
                e.value, COUNT(p.value)
            FROM (SELECT unnest(enum_range(NULL::steward.Pref)) AS value) e
            LEFT JOIN steward.preference p
            ON p.value = e.value AND map_uid = $1
            GROUP BY e.value
        "#;
        let rows = conn.query(stmt, &[&map_uid]).await?;

        let mut counts = Vec::<(PreferenceValue, i64)>::with_capacity(3);
        for row in rows {
            counts.push((row.get("value"), row.get("count")));
        }
        Ok(counts)
    }

    async fn upsert_preference(&self, pref: &Preference) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.preference (player_login, map_uid, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (player_login, map_uid)
            DO UPDATE SET value = excluded.value
        "#;
        let _ = conn
            .execute(stmt, &[&pref.player_login, &pref.map_uid, &pref.value])
            .await?;
        Ok(())
    }

    async fn map_rankings(&self) -> Result<Vec<MapRank>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT
                r.map_uid,
                p.login,
                p.display_name,
                RANK () OVER (
                    PARTITION BY r.map_uid
                    ORDER BY r.millis ASC
                ) pos,
                COUNT(*) OVER (PARTITION BY r.map_uid) max_pos,
                m.in_playlist
            FROM steward.record r
            INNER JOIN steward.player p ON r.player_login = p.login
            INNER JOIN steward.map m ON r.map_uid = m.uid
        "#;
        let rows = conn.query(stmt, &[]).await?;
        Ok(rows
            .iter()
            .map(|row| MapRank {
                map_uid: row.get("map_uid"),
                player_login: row.get("login"),
                player_display_name: DisplayString::from(row.get("display_name")),
                pos: row.get("pos"),
                max_pos: row.get("max_pos"),
                in_playlist: row.get("in_playlist"),
            })
            .collect())
    }

    async fn delete_player(&self, player_login: &str) -> Result<Option<Player>> {
        let mut conn = self.0.get().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.preference WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.sector WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.record WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.player WHERE login = $1 RETURNING *";
        let maybe_row = transaction.query_opt(stmt, &[&player_login]).await?;
        let maybe_player = maybe_row.map(Player::from);

        transaction.commit().await?;
        Ok(maybe_player)
    }

    async fn delete_map(&self, map_uid: &str) -> Result<Option<Map>> {
        let mut conn = self.0.get().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.preference WHERE map_uid = $1";
        let _ = transaction.execute(stmt, &[&map_uid]).await?;

        let stmt = "DELETE FROM steward.sector WHERE map_uid = $1";
        let _ = transaction.execute(stmt, &[&map_uid]).await?;

        let stmt = "DELETE FROM steward.record WHERE map_uid = $1";
        let _ = transaction.execute(stmt, &[&map_uid]).await?;

        let stmt = "DELETE FROM steward.map WHERE uid = $1 RETURNING *";
        let maybe_row = transaction.query_opt(stmt, &[&map_uid]).await?;
        let maybe_map = maybe_row.map(Map::from);

        transaction.commit().await?;
        Ok(maybe_map)
    }
}

impl From<Row> for Map {
    fn from(row: Row) -> Self {
        Map {
            uid: row.get("uid"),
            file_name: row.get("file_name"),
            name: DisplayString::from(row.get("name")),
            author_login: row.get("author_login"),
            author_display_name: DisplayString::from(row.get("author_display_name")),
            author_millis: row.get("author_millis"),
            added_since: row.get("added_since"),
            in_playlist: row.get("in_playlist"),
            exchange_id: row.get("exchange_id"),
        }
    }
}

impl From<Row> for MapEvidence {
    fn from(row: Row) -> Self {
        MapEvidence {
            data: row.get("file"),
            metadata: Map::from(row),
        }
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

impl From<Row> for Preference {
    fn from(row: Row) -> Self {
        Preference {
            player_login: row.get("player_login"),
            map_uid: row.get("map_uid"),
            value: row.get("value"),
        }
    }
}
