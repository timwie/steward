use std::convert::TryFrom;
use std::str::FromStr;
use std::time::Duration;

use bb8::PooledConnection;
use bb8_postgres::PostgresConnectionManager;
use chrono::NaiveDateTime;
use include_dir::{include_dir, Dir};
use tokio_postgres::{NoTls, Row};

use crate::database::structs::*;
use crate::database::DatabaseClient;
use crate::server::{DisplayString, PlayerInfo};

pub async fn pg_connect(conn: &str, timeout: Duration) -> Option<DatabaseClient> {
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

    Some(DatabaseClient::Postgres(pool))
}

/// A connection pool that maintains a set of open connections to the database,
/// handing them out for repeated use.
pub(super) type Pool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub type Error = bb8::RunError<tokio_postgres::Error>;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(not(feature = "unit_test"))]
impl DatabaseClient {
    async fn conn(&self) -> Result<PooledConnection<'_, PostgresConnectionManager<NoTls>>> {
        match self {
            DatabaseClient::Postgres(pool) => Ok(pool.get().await?),
        }
    }

    #[cfg(feature = "integration_test")]
    #[allow(dead_code)]
    pub async fn clear(&self) -> Result<()> {
        let conn = self.conn().await?;
        let _ = conn
            .execute("DROP SCHEMA IF EXISTS steward CASCADE", &[])
            .await?;
        Ok(())
    }

    /// Check for pending database migrations and execute them.
    pub async fn migrate(&self) -> Result<()> {
        // Include all migration statements at compile-time:
        static MIGRATION_DIR: Dir = include_dir!("src/res/migrations/");

        let stmts = |nb: usize| {
            MIGRATION_DIR
                .get_file(format!("{}.sql", nb))
                .and_then(|f| f.contents_utf8())
                .unwrap_or_else(|| panic!("failed to find statements for migration {}", nb))
        };

        let mut conn = self.conn().await?;
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

    /// Return the specified player, or `None` if no such player exists in the database.
    pub async fn player(&self, login: &str) -> Result<Option<Player>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = $1
        "#;
        let row = conn.query_opt(stmt, &[&login]).await?;
        Ok(row.map(Player::from))
    }

    /// Return players for every input login that exists in the database.
    pub async fn players(&self, logins: Vec<&str>) -> Result<Vec<Player>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT *
            FROM steward.player
            WHERE login = ANY($1::text[])
        "#;
        let rows = conn.query(stmt, &[&logins]).await?;
        Ok(rows.into_iter().map(Player::from).collect())
    }

    /// Insert a player into the database.
    /// Update their display name if the player already exists.
    pub async fn upsert_player(&self, player: &PlayerInfo) -> Result<()> {
        let conn = self.conn().await?;
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

    /// Update a player's history, setting *now* as the time they most recently
    /// played the specified map.
    pub async fn add_history(
        &self,
        player_login: &str,
        map_uid: &str,
        last_played: &NaiveDateTime,
    ) -> Result<()> {
        let conn = self.conn().await?;
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

    /// Returns the specified player's history for every specified map they have played.
    ///
    /// # Arguments
    /// `player_login` - a player's login
    /// `map_uids` - A list of map UIDs to return the history for. Use an empty list to select
    ///              records for all maps.
    pub async fn history(&self, player_login: &str, map_uids: Vec<&str>) -> Result<Vec<History>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT
                m.uid map_uid,
                h.last_played,
                RANK () OVER (
                    ORDER BY h.last_played DESC NULLS LAST
                ) - 1 nb_maps_since
            FROM steward.map m
            LEFT JOIN steward.history h ON
                m.uid = h.map_uid
                AND (CARDINALITY($2::text[]) = 0 OR m.uid = ANY($2::text[]))
            WHERE
                h.player_login is NULL
                OR h.player_login = $1
        "#;
        let rows = conn.query(stmt, &[&player_login, &map_uids]).await?;
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

    /// Return the `*.Map.Gbx` file contents of the specified map.
    pub async fn map_file(&self, uid: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT file
            FROM steward.map_file
            WHERE map_uid = $1
        "#;
        let maybe_row = conn.query_opt(stmt, &[&uid]).await?;
        Ok(maybe_row.map(|row| row.get(0)))
    }

    /// Return the specified maps.
    pub async fn maps(&self, map_uids: Vec<&str>) -> Result<Vec<Map>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE CARDINALITY($1::text[]) = 0 OR uid = ANY($1::text[])
        "#;
        let rows = conn.query(stmt, &[&map_uids]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    /// Return the specified map, or `None` if no such map exists in the database.
    pub async fn map(&self, map_uid: &str) -> Result<Option<Map>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE uid = $1
        "#;
        let row = conn.query_opt(stmt, &[&map_uid]).await?;
        Ok(row.map(Map::from))
    }

    /// Insert a map into the database.
    ///
    /// If the given map already exists in the database, update
    ///  - its file
    ///  - its file path
    ///  - its exchange ID.
    pub async fn upsert_map(&self, metadata: &Map, data: Vec<u8>) -> Result<()> {
        let mut conn = self.conn().await?;

        let txn = conn.transaction().await?;

        let stmt = r#"
            INSERT INTO steward.map
                (uid, file_name, name,
                 author_login, author_display_name, author_millis,
                 added_since, exchange_id)
            VALUES
                ($1, $2, $3,
                 $4, $5, $6,
                 $7, $8)
            ON CONFLICT (uid)
            DO UPDATE SET
                file_name = excluded.file_name,
                exchange_id = COALESCE(excluded.exchange_id, steward.map.exchange_id)
        "#;
        let _ = txn
            .execute(
                stmt,
                &[
                    &metadata.uid,
                    &metadata.file_name,
                    &metadata.name.formatted.trim(),
                    &metadata.author_login,
                    &metadata.author_display_name.formatted.trim(),
                    &metadata.author_millis,
                    &metadata.added_since,
                    &metadata.exchange_id,
                ],
            )
            .await?;

        let stmt = r#"
            INSERT INTO steward.map_file (map_uid, file)
            VALUES ($1, $2)
            ON CONFLICT (uid)
            DO UPDATE SET file = excluded.file
        "#;
        let _ = txn.execute(stmt, &[&metadata.uid, &data]).await?;

        txn.commit().await?;
        Ok(())
    }

    /// Return the number of players that have set a record on the specified map,
    /// with the specified lap count.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to count flying lap records.
    pub async fn nb_records(&self, map_uid: &str, nb_laps: i32) -> Result<i64> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT COUNT(*) AS INTEGER
            FROM steward.record
            WHERE map_uid = $1 AND nb_laps = $2;
        "#;
        let row = conn.query_one(stmt, &[&map_uid, &nb_laps]).await?;
        Ok(row.get(0))
    }

    /// Return records on the specified maps, set by the specified players, with the specified
    /// amount of laps.
    ///
    /// # Arguments
    /// `map_uids` - A list of map UIDs to return records for. Use an empty list to select
    ///              records for all maps.
    /// `player_logins` - A list of player logins to return records for. Use an empty list to
    ///                   select records set by any player.
    /// `nb_laps` - The number of required laps. Use `0` if the map is not multi-lap,
    ///             or to get flying lap records.
    /// `limit_per_map` - The maximum number of records returned for each specified map.
    pub async fn records(
        &self,
        map_uids: Vec<&str>,
        player_logins: Vec<&str>,
        nb_laps: i32,
        limit_per_map: Option<i64>,
    ) -> Result<Vec<Record>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT
                r.map_uid, r.pos, r.millis, r.timestamp,
                p.login, p.display_name
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
                WHERE
                    nb_laps = $3
                    AND (CARDINALITY($1::text[]) = 0 OR map_uid = ANY($1::text[]))
                    AND (CARDINALITY($2::text[]) = 0 OR player_login = ANY($2::text[]))
                LIMIT $4
            ) r
            INNER JOIN steward.player p ON r.player_login = p.login
        "#;
        let rows = conn
            .query(stmt, &[&map_uids, &player_logins, &nb_laps, &limit_per_map])
            .await?;
        let records = rows
            .into_iter()
            .map(|row| Record {
                map_uid: row.get("map_uid"),
                player_login: row.get("login"),
                nb_laps,
                map_rank: row.get("pos"),
                player_display_name: DisplayString::from(row.get("display_name")),
                timestamp: row.get("timestamp"),
                millis: row.get("millis"),
            })
            .collect();
        Ok(records)
    }

    /// Return the top record set by any player on the specified map,
    /// with the specified lap count, or `None` if no player has completed such a
    /// run on that map.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get the top flying lap records.
    pub async fn top_record(&self, map_uid: &str, nb_laps: i32) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(1))
            .await?
            .into_iter()
            .next())
    }

    /// Return limited number of top records on the specified map,
    /// with the specified lap count, sorted from best to worse.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get flying lap records.
    pub async fn top_records(
        &self,
        map_uid: &str,
        limit: i64,
        nb_laps: i32,
    ) -> Result<Vec<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(limit))
            .await?)
    }

    /// Return the personal best of the specified player on the specified map,
    /// with the specified lap count, or `None` if the player has not completed such a
    /// run on that map.
    ///
    /// Use `nb_laps = 0` if the map is not multi-lap, or to get the player's flying lap PB.
    pub async fn player_record(
        &self,
        map_uid: &str,
        player_login: &str,
        nb_laps: i32,
    ) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![player_login], nb_laps, None)
            .await?
            .into_iter()
            .next())
    }

    /// Return the number of players that have set a record on at least one map.
    pub async fn nb_players_with_record(&self) -> Result<i64> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT COUNT(DISTINCT player_login)
            FROM steward.record
        "#;
        let row = conn.query_one(stmt, &[]).await?;
        Ok(row.get(0))
    }

    /// List all map UIDs that the specified player has not completed a run on.
    pub async fn maps_without_player_record(&self, player_login: &str) -> Result<Vec<String>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT DISTINCT m.uid
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

    /// Without inserting the given record, return the map rank it would achieve,
    /// if it were inserted.
    pub async fn record_preview(&self, record: &RecordEvidence) -> Result<i64> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT COUNT(*)
            FROM steward.record r
            WHERE
                map_uid = $1
                AND r.nb_laps = $2
                AND r.player_login != $3
                AND r.millis < $4
        "#;
        let row = conn
            .query_one(
                stmt,
                &[
                    &record.map_uid,
                    &record.nb_laps,
                    &record.player_login,
                    &record.millis,
                ],
            )
            .await?;
        Ok(1 + row.get::<usize, i64>(0))
    }

    /// Updates the player's personal best on a map.
    ///
    /// # Note
    /// If a previous record exists for that player, this function does not
    /// check if the given record is actually better than the one in the database.
    pub async fn upsert_record(&self, rec: &RecordEvidence) -> Result<()> {
        let conn = self.conn().await?;

        let stmt = r#"
            INSERT INTO steward.record
                (player_login, map_uid, nb_laps, millis, timestamp)
            VALUES
                ($1, $2, $3, $4, $5)
            ON CONFLICT (player_login, map_uid, nb_laps)
            DO UPDATE SET
                millis = excluded.millis,
                timestamp = excluded.timestamp
        "#;

        let _ = conn
            .execute(
                stmt,
                &[
                    &rec.player_login,
                    &rec.map_uid,
                    &rec.nb_laps,
                    &rec.millis,
                    &rec.timestamp,
                ],
            )
            .await?;

        Ok(())
    }

    /// List all preferences that the specified player has set.
    pub async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT * FROM steward.preference
            WHERE player_login = $1 AND value IS NOT NULL
        "#;
        let rows = conn.query(stmt, &[&player_login]).await?;
        let prefs = rows.into_iter().map(Preference::from).collect();
        Ok(prefs)
    }

    /// Count the number of times each preference was set by any player, for the specified map.
    pub async fn count_map_preferences(
        &self,
        map_uid: &str,
    ) -> Result<Vec<(PreferenceValue, i64)>> {
        let conn = self.conn().await?;
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

    /// Insert a player's map preference, overwriting any previous preference.
    pub async fn upsert_preference(&self, pref: &Preference) -> Result<()> {
        let conn = self.conn().await?;
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

    /// Calculate the map rank of *every* player, for each of the specified maps.
    ///
    /// For multi-lap maps, the best map rank will have the best flying lap.
    ///
    /// # Note
    /// The length of this collection is equal to the total number of `nb_laps == 0` records
    /// stored in the database. This function should only be used when calculating
    /// the server ranking.
    pub async fn map_rankings(&self, map_uids: Vec<&str>) -> Result<Vec<MapRank>> {
        let conn = self.conn().await?;
        let stmt = r#"
            SELECT
                r.map_uid,
                p.login,
                p.display_name,
                RANK () OVER (
                    PARTITION BY r.map_uid
                    ORDER BY r.millis ASC
                ) pos,
                COUNT(*) OVER (PARTITION BY r.map_uid) max_pos
            FROM steward.record r
            INNER JOIN steward.player p ON r.player_login = p.login
            INNER JOIN steward.map m ON
                r.map_uid = m.uid
                AND (CARDINALITY($1::text[]) = 0 OR r.map_uid = ANY($1::text[]))
            WHERE r.nb_laps = 0
        "#;
        let rows = conn.query(stmt, &[&map_uids]).await?;
        Ok(rows
            .iter()
            .map(|row| MapRank {
                map_uid: row.get("map_uid"),
                player_login: row.get("login"),
                player_display_name: DisplayString::from(row.get("display_name")),
                pos: row.get("pos"),
                max_pos: row.get("max_pos"),
            })
            .collect())
    }

    /// Delete a player, their preferences, and their records.
    /// The data is lost forever.
    pub async fn delete_player(&self, player_login: &str) -> Result<Option<Player>> {
        let mut conn = self.conn().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.preference WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.record WHERE player_login = $1";
        let _ = transaction.execute(stmt, &[&player_login]).await?;

        let stmt = "DELETE FROM steward.player WHERE login = $1 RETURNING *";
        let maybe_row = transaction.query_opt(stmt, &[&player_login]).await?;
        let maybe_player = maybe_row.map(Player::from);

        transaction.commit().await?;
        Ok(maybe_player)
    }

    /// Delete a map, its preferences, and its records.
    /// The data is lost forever.
    pub async fn delete_map(&self, map_uid: &str) -> Result<Option<Map>> {
        let mut conn = self.conn().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.preference WHERE map_uid = $1";
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
            exchange_id: row.get("exchange_id"),
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
