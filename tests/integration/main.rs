use std::sync::Arc;

use anyhow::Result;
use chrono::{NaiveDateTime, SubsecRound, Utc};
use testcontainers::*;

use steward::database::*;
use steward::server::{GameString, PlayerInfo};

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
    let expected_info = player_info("login", "nickname");
    let expected = Player {
        login: "login".to_string(),
        nick_name: GameString::from("nickname".to_string()),
    };

    db.upsert_player(&expected_info).await?;
    let actual = db.player("login").await?;

    assert_eq!(Some(expected), actual);
    Ok(())
}

#[tokio::test]
async fn test_player_update() -> Result<()> {
    let db = clean_db().await?;
    let old_info = player_info("login", "nickname");
    let new_info = player_info("login", "new nickname");
    let expected = Player {
        login: "login".to_string(),
        nick_name: GameString::from("new nickname".to_string()),
    };

    db.upsert_player(&old_info).await?;
    db.upsert_player(&new_info).await?;
    let actual = db.player("login").await?;

    assert_eq!(Some(expected), actual);
    Ok(())
}

#[tokio::test]
async fn test_nb_records_zero() -> Result<()> {
    let db = clean_db().await?;
    assert_eq!(0, db.nb_records("uid1").await?);

    let map = map_evidence("uid1", "file1");
    db.upsert_map(&map).await?;
    assert_eq!(0, db.nb_records("uid1").await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_one() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map_evidence("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map).await?;
    db.upsert_record(&rec).await?;

    assert_eq!(1, db.nb_records("uid1").await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_multiple_maps() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let rec11 = record_evidence("login1", "uid1", 10000);
    let rec12 = record_evidence("login2", "uid1", 10000);
    let rec21 = record_evidence("login1", "uid2", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_record(&rec11).await?;
    db.upsert_record(&rec12).await?;
    db.upsert_record(&rec21).await?;

    assert_eq!(2, db.nb_records("uid1").await?);
    assert_eq!(1, db.nb_records("uid2").await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_one_per_player() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map_evidence("uid1", "file1");
    let rec1 = record_evidence("login", "uid1", 10000);
    let rec2 = record_evidence("login", "uid1", 9000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    assert_eq!(1, db.nb_records("uid1").await?);

    Ok(())
}

// TODO test .upsert_map insert
// TODO test .upsert_map update filename, exchangeid
// TODO test .upsert_map update does not set NULL exchangeid
// => assert with .map, .maps, .map_files, .playlist

// TODO test .playlist_add & .playlist_remove
// => assert with .playlist

// TODO test .top_record
// TODO test .top_records
// TODO test .nb_players_with_record
// TODO test .maps_without_player_record
// TODO test .players_without_map_record
// TODO test .record_preview
// TODO test .map_rankings

// TODO test .player_preferences
// TODO test .map_preferences
// TODO test .count_map_preferences
// TODO test .upsert_preference

// TODO test delete_player
// TODO test delete_map
// TODO test delete_old_ghosts

#[tokio::test]
async fn test_player_record_some() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map_evidence("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map).await?;
    db.upsert_record(&rec).await?;

    let expected = record_detailed(1, "nickname", rec);
    let expected = Some(expected);

    let actual = db.player_record("uid1", "login").await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_player_records_single() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map_evidence("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map).await?;
    db.upsert_record(&rec).await?;

    let expected = record_detailed(1, "nickname", rec);
    let expected = vec![expected];

    let actual = db.player_records("uid1", vec!["login"]).await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_player_records_multiple_players() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map = map_evidence("uid1", "file1");
    let rec1 = record_evidence("login1", "uid1", 9000);
    let rec2 = record_evidence("login2", "uid1", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    let expected1 = record_detailed(1, "nickname1", rec1);
    let expected2 = record_detailed(2, "nickname2", rec2);
    let expected = vec![expected1, expected2];

    let actual = db.player_records("uid1", vec!["login1", "login2"]).await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_player_records_multiple_maps() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid2", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    let expected = record_detailed(1, "nickname1", rec1);
    let expected = vec![expected];

    let actual = db.player_records("uid1", vec!["login1", "login2"]).await?;
    assert_eq!(expected, actual);

    Ok(())
}

fn player_info(login: &str, nick_name: &str) -> PlayerInfo {
    PlayerInfo {
        uid: 0,
        login: login.to_string(),
        nick_name: GameString::from(nick_name.to_string()),
        flag_digit_mask: 101_000_000,
        spectator_digit_mask: 2_551_010,
    }
}

fn map_evidence(uid: &str, file_name: &str) -> MapEvidence {
    MapEvidence {
        metadata: Map {
            uid: uid.to_string(),
            file_name: file_name.to_string(),
            name: GameString::from("".to_string()),
            author_login: "".to_string(),
            added_since: now(),
            in_playlist: true,
            exchange_id: None,
        },
        data: "map file".as_bytes().to_owned(),
    }
}

fn record_evidence(login: &str, map_uid: &str, millis: i32) -> RecordEvidence {
    RecordEvidence {
        player_login: login.to_string(),
        map_uid: map_uid.to_string(),
        millis,
        timestamp: now(),
        validation: "validation replay".as_bytes().to_owned(),
        ghost: Some("ghost replay".as_bytes().to_owned()),
        sectors: (0..5)
            .map(|i| RecordSector {
                index: i,
                cp_millis: (i + 1) * (millis / 5),
                cp_speed: 420.1337,
                cp_distance: 1337.42 + i as f32,
            })
            .collect(),
    }
}

fn record_detailed(pos: i64, nick_name: &str, ev: RecordEvidence) -> RecordDetailed {
    RecordDetailed {
        map_rank: pos,
        player_login: ev.player_login,
        player_nick_name: GameString::from(nick_name.to_string()),
        millis: ev.millis,
        timestamp: ev.timestamp,
        cp_millis: ev.sectors.iter().map(|sector| sector.cp_millis).collect(),
    }
}

fn now() -> NaiveDateTime {
    // If we want to test date equality, we need to round at least a few nano
    // digits, since we lose some precision when storing in the database, f.e.
    //      before: 2020-06-06T19:49:59.973170303
    //       after: 2020-06-06T19:49:59.973170
    Utc::now().naive_utc().round_subsecs(5)
}
