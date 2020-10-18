use async_trait::async_trait;

use crate::database::api::{Record, RecordEvidence, RecordQueries};
use crate::database::{DatabaseClient, Result};
use crate::server::DisplayString;

#[async_trait]
impl RecordQueries for DatabaseClient {
    async fn records(
        &self,
        map_uids: Vec<&str>,
        player_logins: Vec<&str>,
        nb_laps: i32,
        limit_per_map: Option<i64>,
    ) -> Result<Vec<Record>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT
                r.map_uid, r.pos, r.max_pos, r.millis, r.timestamp, r.cp_millis,
                p.login, p.display_name
            FROM (
                SELECT
                   *,
                   RANK () OVER (
                      PARTITION BY map_uid
                      ORDER BY millis ASC
                   ) pos,
                   COUNT(*) OVER (
                      PARTITION BY map_uid
                   ) max_pos
                FROM steward.record
                WHERE
                    nb_laps = $3
                    AND (CARDINALITY($1::text[]) = 0 OR map_uid = ANY($1::text[]))
                LIMIT $4
            ) r
            INNER JOIN steward.player p ON
                r.player_login = p.login
                AND (CARDINALITY($2::text[]) = 0 OR p.login = ANY($2::text[]))
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
                max_map_rank: row.get("max_pos"),
                player_display_name: DisplayString::from(row.get("display_name")),
                timestamp: row.get("timestamp"),
                millis: row.get("millis"),
                cp_millis: row.get("cp_millis"),
            })
            .collect();
        Ok(records)
    }

    async fn top_record(&self, map_uid: &str, nb_laps: i32) -> Result<Option<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(1))
            .await?
            .into_iter()
            .next())
    }

    async fn top_records(&self, map_uid: &str, limit: i64, nb_laps: i32) -> Result<Vec<Record>> {
        Ok(self
            .records(vec![map_uid], vec![], nb_laps, Some(limit))
            .await?)
    }

    async fn player_record(
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

    async fn nb_players_with_record(&self) -> Result<i64> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT COUNT(DISTINCT player_login)
            FROM steward.record
        "#;
        let row = conn.query_one(stmt, &[]).await?;
        Ok(row.get(0))
    }

    async fn maps_without_player_record(&self, player_login: &str) -> Result<Vec<String>> {
        let conn = self.pool.get().await?;
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

    async fn record_preview(&self, record: &RecordEvidence) -> Result<i64> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT COUNT(*)
            FROM steward.record
            WHERE
                map_uid = $1
                AND nb_laps = $2
                AND player_login != $3
                AND millis < $4
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

    async fn upsert_record(&self, rec: &RecordEvidence) -> Result<()> {
        let conn = self.pool.get().await?;

        let stmt = r#"
            INSERT INTO steward.record
                (player_login, map_uid, nb_laps, millis, timestamp, cp_millis)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (player_login, map_uid, nb_laps)
            DO UPDATE SET
                millis = excluded.millis,
                cp_millis = excluded.cp_millis,
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
                    &rec.cp_millis,
                ],
            )
            .await?;

        Ok(())
    }
}
