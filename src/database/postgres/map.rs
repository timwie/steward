use async_trait::async_trait;
use tokio_postgres::Row;

use crate::database::api::{Map, MapQueries, RemovedMap};
use crate::database::{DatabaseClient, Result};
use crate::server::DisplayString;

#[async_trait]
impl MapQueries for DatabaseClient {
    async fn map_file(&self, uid: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT file
            FROM steward.map_file
            WHERE map_uid = $1
        "#;
        let maybe_row = conn.query_opt(stmt, &[&uid]).await?;
        Ok(maybe_row.map(|row| row.get(0)))
    }

    async fn maps(&self, map_uids: Vec<&str>) -> Result<Vec<Map>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE
                file_name IS NOT NULL
                AND CARDINALITY($1::text[]) = 0 OR uid = ANY($1::text[])
        "#;
        let rows = conn.query(stmt, &[&map_uids]).await?;
        let maps = rows.into_iter().map(Map::from).collect();
        Ok(maps)
    }

    async fn map(&self, map_uid: &str) -> Result<Option<Map>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE
                uid = $1
                AND file_name IS NOT NULL
        "#;
        let row = conn.query_opt(stmt, &[&map_uid]).await?;
        Ok(row.map(Map::from))
    }

    async fn upsert_map(&self, metadata: &Map, data: Vec<u8>) -> Result<()> {
        let mut conn = self.pool.get().await?;

        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE file_name = $1
        "#;
        let row = conn.query_opt(stmt, &[&metadata.file_name]).await?;
        let overwritten_map = row.map(Map::from);

        let txn = conn.transaction().await?;

        if overwritten_map.is_some() {
            let stmt = r#"
                UPDATE steward.map
                SET file_name = NULL
                WHERE file_name = $1
            "#;
            let _ = txn.execute(stmt, &[&metadata.file_name]).await?;
        }

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
            ON CONFLICT (map_uid)
            DO UPDATE SET file = excluded.file
        "#;
        let _ = txn.execute(stmt, &[&metadata.uid, &data]).await?;

        txn.commit().await?;
        Ok(())
    }

    async fn delete_map(&self, map_uid: &str) -> Result<Option<RemovedMap>> {
        let mut conn = self.pool.get().await?;
        let transaction = conn.transaction().await?;

        let stmt = "DELETE FROM steward.ta_preference WHERE map_uid = $1";
        let _ = transaction.execute(stmt, &[&map_uid]).await?;

        let stmt = "DELETE FROM steward.record WHERE map_uid = $1";
        let _ = transaction.execute(stmt, &[&map_uid]).await?;

        let stmt = "DELETE FROM steward.map WHERE uid = $1 RETURNING *";
        let maybe_row = transaction.query_opt(stmt, &[&map_uid]).await?;
        let maybe_map = maybe_row.map(RemovedMap::from);

        transaction.commit().await?;
        Ok(maybe_map)
    }

    async fn removed_maps(&self) -> Result<Vec<RemovedMap>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT *
            FROM steward.map
            WHERE file_name IS NULL
        "#;
        let rows = conn.query(stmt, &[]).await?;
        let maps = rows.into_iter().map(RemovedMap::from).collect();
        Ok(maps)
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

impl From<Row> for RemovedMap {
    fn from(row: Row) -> Self {
        RemovedMap {
            uid: row.get("uid"),
            file_name: row.get("file_name"),
            name: DisplayString::from(row.get("name")),
            author_login: row.get("author_login"),
            author_display_name: DisplayString::from(row.get("author_display_name")),
            exchange_id: row.get("exchange_id"),
        }
    }
}
