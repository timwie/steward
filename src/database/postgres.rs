use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Result;
use include_dir::Dir;
use tokio_postgres::Row;

use async_trait::async_trait;

use crate::config::Config;
use crate::database::queries::Queries;
use crate::database::structs::*;
use crate::ingame::{MapInfo, PlayerInfo};

/// Connect to the Postgres database and open a connection pool.
pub async fn pg_connect(config: &Config) -> Arc<dyn Queries> {
    let config = tokio_postgres::config::Config::from_str(&config.postgres_connection)
        .expect("failed to parse postgres connection string");

    log::debug!("using postgres connection config:");
    log::debug!("{:?}", config);

    let pg_mgr = bb8_postgres::PostgresConnectionManager::new(config, tokio_postgres::NoTls);

    let pool = bb8::Pool::builder()
        .build(pg_mgr)
        .await
        .expect("failed to build database pool");

    Arc::new(PostgresClient(pool)) as Arc<dyn Queries>
}

/// A connection pool that maintains a set of open
/// connections to the database, handing them out for
/// repeated use.
type PostgresPool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

/// `Queries` implementation that produces `bb8::RunError<tokio_postgres::Error>s`.
#[derive(Clone)]
struct PostgresClient(PostgresPool);

impl PostgresClient {
    async fn playlist_edit(&self, map_uid: &str, in_playlist: bool) -> Result<Option<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            UPDATE steward.map
            SET in_playlist = $2
            WHERE uid = $1
            RETURNING uid, file_name, name, author_login, added_since, in_playlist, exchange_id
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

    async fn upsert_player(&self, player: &PlayerInfo) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.player
                (uid, login, nick_name)
            VALUES
                ($1, $2, $3)
            ON CONFLICT (uid)
            DO UPDATE SET
                nick_name = excluded.nick_name
        "#;
        let _ = conn
            .execute(stmt, &[&player.uid, &player.login, &player.nick_name])
            .await?;
        Ok(())
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
            SELECT uid, file_name, name, author_login, added_since, in_playlist, exchange_id
            FROM steward.map
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    async fn nb_maps(&self) -> Result<i64> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(*)
            FROM steward.map
        "#;
        let row = conn.query_one(stmt, &[]).await?;
        Ok(row.get(0))
    }

    async fn playlist(&self) -> Result<Vec<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT uid, file_name, name, author_login, added_since, in_playlist, exchange_id
            FROM steward.map
            WHERE in_playlist = True
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    async fn map(&self, map_uid: &str) -> Result<Option<Map>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT uid, file_name, name, author_login, added_since, in_playlist, exchange_id
            FROM steward.map
            WHERE uid = $1
        "#;
        let row = conn.query_opt(stmt, &[&map_uid]).await?;
        Ok(row.map(Map::from))
    }

    async fn insert_map(&self, map: &Map, data: Vec<u8>) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.map
                (uid, file_name, file,
                 name, author_login, added_since,
                 in_playlist, exchange_id)
            VALUES
                ($1, $2, $3,
                 $4, $5, $6,
                 $7, $8)
            ON CONFLICT (uid) DO NOTHING
        "#;
        let _ = conn
            .execute(
                stmt,
                &[
                    &map.uid,
                    &map.file_name,
                    &data,
                    &map.name,
                    &map.author_login,
                    &map.added_since,
                    &map.in_playlist,
                    &map.exchange_id,
                ],
            )
            .await?;
        Ok(())
    }

    async fn upsert_map(&self, map: &MapInfo, data: Vec<u8>) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.map
                (uid, file_name, file,
                 name, author_login, added_since,
                 in_playlist)
            VALUES
                ($1, $2, $3,
                 $4, $5, $6,
                 $7)
            ON CONFLICT (uid)
            DO UPDATE SET file_name = excluded.file_name
        "#;
        let _ = conn
            .execute(
                stmt,
                &[
                    &map.uid,
                    &map.file_name,
                    &data,
                    &map.name,
                    &map.author_login,
                    &SystemTime::now(),
                    &true,
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

    async fn top_record(&self, map_uid: &str) -> Result<Option<RecordDetailed>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT r.player_uid, p.nick_name, r.timestamp, s.cp_millis
            FROM (
                SELECT player_uid, timestamp
                FROM steward.record
                WHERE map_uid = $1
                ORDER BY millis ASC
                LIMIT 1
            ) r
            RIGHT JOIN steward.sector s ON r.player_uid = s.player_uid
            LEFT JOIN steward.player p ON r.player_uid = p.uid
            WHERE r.player_uid IS NOT NULL AND s.map_uid = $1
            ORDER BY s.index ASC
        "#;
        let rows = conn.query(stmt, &[&map_uid]).await?;

        let mut sector_times = Vec::new();
        let mut offset_millis = 0;
        for row in &rows {
            let cp_millis: i32 = row.get("cp_millis");
            sector_times.push(cp_millis - offset_millis);
            offset_millis = cp_millis;
        }

        Ok(rows.first().map(|row| RecordDetailed {
            map_rank: 1,
            player_uid: row.get("player_uid"),
            player_nick_name: row.get("nick_name"),
            timestamp: row.get("timestamp"),
            millis: offset_millis,
            sector_times,
        }))
    }

    async fn top_records(&self, map_uid: &str, limit: i64) -> Result<Vec<Record>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT r.player_uid, p.nick_name, r.timestamp, r.millis
            FROM steward.record r
            LEFT JOIN steward.player p
            ON r.player_uid = p.uid
            WHERE map_uid = $1
            ORDER BY r.millis ASC
            LIMIT $2
        "#;
        let rows = conn.query(stmt, &[&map_uid, &limit]).await?;

        Ok(rows
            .iter()
            .map(|row| Record {
                player_uid: row.get("player_uid"),
                player_nick_name: row.get("nick_name"),
                timestamp: row.get("timestamp"),
                millis: row.get("millis"),
            })
            .collect())
    }

    async fn player_record(
        &self,
        map_uid: &str,
        player_uid: i32,
    ) -> Result<Option<RecordDetailed>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT r.pos, p.nick_name, r.timestamp, s.cp_millis
            FROM (
                SELECT
                   player_uid,
                   timestamp,
                   RANK () OVER (
                      ORDER BY millis
                   ) pos
                FROM steward.record
                WHERE map_uid = $1
            ) r
            RIGHT JOIN steward.sector s ON r.player_uid = s.player_uid
            LEFT JOIN steward.player p ON r.player_uid = p.uid
            WHERE map_uid = $1 AND r.player_uid = $2
            ORDER BY s.index ASC
        "#;
        let rows = conn.query(stmt, &[&map_uid, &player_uid]).await?;

        let mut sector_times = Vec::new();
        let mut offset_millis = 0;
        for row in &rows {
            let cp_millis: i32 = row.get("cp_millis");
            sector_times.push(cp_millis - offset_millis);
            offset_millis = cp_millis;
        }

        Ok(rows.first().map(|row| RecordDetailed {
            map_rank: row.get("pos"),
            player_uid,
            player_nick_name: row.get("nick_name"),
            timestamp: row.get("timestamp"),
            millis: offset_millis,
            sector_times,
        }))
    }

    async fn nb_players_with_record(&self) -> Result<i64> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(DISTINCT player_uid)
            FROM steward.record
        "#;
        let row = conn.query_one(stmt, &[]).await?;
        Ok(row.get(0))
    }

    async fn maps_without_player_record(&self, player_uid: i32) -> Result<Vec<String>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT m.uid
            FROM steward.map m
            LEFT JOIN (
                SELECT map_uid FROM steward.record WHERE player_uid = $1
            ) r
            ON m.uid = r.map_uid
            WHERE r.map_uid IS NULL
        "#;
        let rows = conn.query(stmt, &[&player_uid]).await?;
        let maps = rows.iter().map(|row| row.get(0)).collect();
        Ok(maps)
    }

    async fn players_without_map_record(&self, map_uid: &str) -> Result<Vec<i32>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT p.uid
            FROM steward.player p
            LEFT JOIN (
                SELECT player_uid FROM steward.record WHERE map_uid = $1
            ) r
            ON p.uid = r.player_uid
            WHERE r.player_uid IS NULL
        "#;
        let rows = conn.query(stmt, &[&map_uid]).await?;
        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    async fn record_preview(&self, record: &RecordEvidence) -> Result<i32> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT COUNT(*)
            FROM steward.record
            WHERE map_uid = $1 AND r.player_uid != $2 AND r.millis < $3
        "#;
        let row = conn
            .query_one(stmt, &[&record.map_uid, &record.player_uid, &record.millis])
            .await?;
        Ok(1 + row.get::<usize, i32>(0))
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
                (player_uid, map_uid, millis, validation, ghost, timestamp)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (player_uid, map_uid)
            DO UPDATE SET
                validation = excluded.validation,
                ghost = excluded.ghost,
                timestamp = excluded.timestamp
        "#;

        let _ = transaction
            .execute(
                insert_record_stmt,
                &[
                    &rec.player_uid,
                    &rec.map_uid,
                    &rec.millis,
                    &rec.validation,
                    &rec.ghost,
                    &rec.timestamp,
                ],
            )
            .await?;

        let insert_sector_stmt = r#"
            INSERT INTO steward.sector
                (player_uid, map_uid, index, cp_millis, cp_speed, cp_distance)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (player_uid, map_uid, index)
            DO UPDATE SET
                cp_millis = excluded.cp_millis,
                cp_speed = excluded.cp_speed,
                cp_distance = excluded.cp_distance
        "#;

        for sector in &rec.sectors {
            let _ = transaction
                .execute(
                    insert_sector_stmt,
                    &[
                        &rec.player_uid,
                        &rec.map_uid,
                        &sector.index,
                        &sector.cp_millis,
                        &sector.cp_speed,
                        &sector.cp_distance,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    async fn player_preferences(&self, player_uid: i32) -> Result<Vec<Preference>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT * FROM steward.preference
            WHERE player_uid = $1 AND value IS NOT NULL
        "#;
        let rows = conn.query(stmt, &[&player_uid]).await?;
        let prefs = rows.into_iter().map(Preference::from).collect();
        Ok(prefs)
    }

    async fn map_preferences(&self, map_uid: &str) -> Result<Vec<Preference>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT * FROM steward.preference
            WHERE map_uid = $1 AND value IS NOT NULL
        "#;
        let rows = conn.query(stmt, &[&map_uid]).await?;
        let prefs = rows.into_iter().map(Preference::from).collect();
        Ok(prefs)
    }

    async fn count_map_preferences(&self, map_uid: &str) -> Result<HashMap<PreferenceValue, i64>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT value, COUNT(value)
            FROM steward.preference
            WHERE map_uid = $1 AND value IS NOT NULL
            GROUP BY value
        "#;
        let rows = conn.query(stmt, &[&map_uid]).await?;

        let mut counts: HashMap<PreferenceValue, i64> = HashMap::new();
        for row in rows {
            counts.insert(row.get(0), row.get(1));
        }
        Ok(counts)
    }

    async fn upsert_preference(&self, pref: &Preference) -> Result<()> {
        let conn = self.0.get().await?;
        let stmt = r#"
            INSERT INTO steward.preference (player_uid, map_uid, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (player_uid, map_uid)
            DO UPDATE SET value = excluded.value
        "#;
        let _ = conn
            .execute(stmt, &[&pref.player_uid, &pref.map_uid, &pref.value])
            .await?;
        Ok(())
    }

    async fn map_rankings(&self) -> Result<Vec<MapRank>> {
        let conn = self.0.get().await?;
        let stmt = r#"
            SELECT
                r.map_uid,
                r.player_uid,
                p.nick_name,
                RANK () OVER (
                    PARTITION BY r.map_uid
                    ORDER BY r.millis ASC
                ) pos,
                COUNT(*) OVER (PARTITION BY r.map_uid) max_pos
            FROM steward.record r
            LEFT JOIN steward.player p
            ON r.player_uid = p.uid
        "#;
        let rows = conn.query(stmt, &[]).await?;
        Ok(rows
            .iter()
            .map(|row| MapRank {
                map_uid: row.get("map_uid"),
                player_uid: row.get("player_uid"),
                player_nick_name: row.get("nick_name"),
                pos: row.get("pos"),
                max_pos: row.get("max_pos"),
            })
            .collect())
    }
}

impl From<Row> for Map {
    fn from(row: Row) -> Self {
        Map {
            uid: row.get("uid"),
            file_name: row.get("file_name"),
            name: row.get("name"),
            author_login: row.get("author_login"),
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
            uid: row.get("uid"),
            login: row.get("login"),
            nick_name: row.get("nick_name"),
        }
    }
}

impl From<Row> for Preference {
    fn from(row: Row) -> Self {
        Preference {
            player_uid: row.get("player_uid"),
            map_uid: row.get("map_uid"),
            value: row.get("value"),
        }
    }
}
