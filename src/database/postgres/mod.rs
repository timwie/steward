use std::str::FromStr;
use std::time::Duration;

use include_dir::{include_dir, Dir};

mod map;
mod player;
mod record;
mod timeattack;

/// A connection pool that maintains a set of open connections to the database,
/// handing them out for repeated use.
pub(super) type Pool = bb8::Pool<bb8_postgres::PostgresConnectionManager<tokio_postgres::NoTls>>;

pub type Error = bb8::RunError<tokio_postgres::Error>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct DatabaseClient {
    pub(super) pool: Pool,
}

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

    Some(DatabaseClient { pool })
}

impl DatabaseClient {
    #[cfg(feature = "integration_test")]
    #[allow(dead_code)]
    pub async fn clear(&self) -> Result<()> {
        let conn = self.pool.get().await?;
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

        let mut conn = self.pool.get().await?;
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
}
