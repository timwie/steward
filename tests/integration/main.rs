use std::sync::Arc;

use anyhow::Result;
use testcontainers::*;

use gbx::PlayerInfo;
use steward::database::{pg_connect, Database, Player};

/// Spins up a Postgres database in a Docker container.
async fn clean_db() -> Result<Arc<dyn Database>> {
    let docker = clients::Cli::default();

    let db = "postgres-db-test";
    let user = "postgres-user-test";
    let password = "postgres-password-test";

    let generic_postgres = images::generic::GenericImage::new("postgres:latest")
        .with_wait_for(images::generic::WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_DB", db)
        .with_env_var("POSTGRES_USER", user)
        .with_env_var("POSTGRES_PASSWORD", password);

    let container = docker.run(generic_postgres);

    let pg_conn_str = format!(
        "postgres://{}:{}@localhost:{}/{}",
        user,
        password,
        container.get_host_port(5432).unwrap(),
        db
    );

    let client = pg_connect(&pg_conn_str).await;
    let arc = Arc::new(client.clone()) as Arc<dyn Database>;

    let conn = client.0.get().await?;
    conn.execute("DROP SCHEMA IF EXISTS steward CASCADE", &[])
        .await?;
    client.migrate().await?;

    Ok(arc)
}

#[tokio::test]
async fn test_player_insert() -> Result<()> {
    let db = clean_db().await?;
    let expected_info = player_info(0, "login", "nickname");
    let expected = Player {
        login: "login".to_string(),
        nick_name: "nickname".to_string(),
    };

    db.upsert_player(&expected_info).await?;
    let actual = db.player("login").await?;

    assert_eq!(Some(expected), actual);
    Ok(())
}

#[tokio::test]
async fn test_player_update() -> Result<()> {
    let db = clean_db().await?;
    let old_info = player_info(0, "login", "nickname");
    let new_info = player_info(0, "login", "new nickname");
    let expected = Player {
        login: "login".to_string(),
        nick_name: "new nickname".to_string(),
    };

    db.upsert_player(&old_info).await?;
    db.upsert_player(&new_info).await?;
    let actual = db.player("login").await?;

    assert_eq!(Some(expected), actual);
    Ok(())
}

fn player_info(uid: i32, login: &str, nick_name: &str) -> PlayerInfo {
    PlayerInfo {
        uid,
        login: login.to_string(),
        nick_name: nick_name.to_string(),
        flag_digit_mask: 101_000_000,
        spectator_digit_mask: 2_551_010,
    }
}
