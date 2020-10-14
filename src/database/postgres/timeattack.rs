use std::convert::TryFrom;

use async_trait::async_trait;
use chrono::NaiveDateTime;
use tokio_postgres::Row;

use crate::database::timeattack::{
    History, MapRank, Preference, PreferenceValue, TimeAttackQueries,
};
use crate::database::{DatabaseClient, Result};
use crate::server::DisplayString;

#[async_trait]
impl TimeAttackQueries for DatabaseClient {
    async fn add_history(
        &self,
        player_login: &str,
        map_uid: &str,
        last_played: &NaiveDateTime,
    ) -> Result<()> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            INSERT INTO steward.ta_history
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

    async fn history(&self, player_login: &str, map_uids: Vec<&str>) -> Result<Vec<History>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT
                m.uid map_uid,
                h.last_played,
                RANK () OVER (
                    ORDER BY h.last_played DESC NULLS LAST
                ) - 1 nb_maps_since
            FROM steward.map m
            LEFT JOIN steward.ta_history h ON
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

    async fn player_preferences(&self, player_login: &str) -> Result<Vec<Preference>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT * FROM steward.ta_preference
            WHERE player_login = $1 AND value IS NOT NULL
        "#;
        let rows = conn.query(stmt, &[&player_login]).await?;
        let prefs = rows.into_iter().map(Preference::from).collect();
        Ok(prefs)
    }

    async fn count_map_preferences(&self, map_uid: &str) -> Result<Vec<(PreferenceValue, i64)>> {
        let conn = self.pool.get().await?;
        let stmt = r#"
            SELECT
                e.value, COUNT(p.value)
            FROM (SELECT unnest(enum_range(NULL::steward.Pref)) AS value) e
            LEFT JOIN steward.ta_preference p
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
        let conn = self.pool.get().await?;
        let stmt = r#"
            INSERT INTO steward.ta_preference (player_login, map_uid, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (player_login, map_uid)
            DO UPDATE SET value = excluded.value
        "#;
        let _ = conn
            .execute(stmt, &[&pref.player_login, &pref.map_uid, &pref.value])
            .await?;
        Ok(())
    }

    async fn map_rankings(&self, map_uids: Vec<&str>) -> Result<Vec<MapRank>> {
        let conn = self.pool.get().await?;
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
