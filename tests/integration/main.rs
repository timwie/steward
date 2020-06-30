use std::ops::Sub;
use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime, SubsecRound, Utc};
use testcontainers::*;

use steward::database::*;
use steward::server::{GameString, PlayerInfo};

// TODO add database tests
// [x] migrate
// [x] player
// [x] upsert_player
// [x] add_history
// [x] history
// [ ] map_files
// [ ] maps
// [ ] playlist
// [ ] map
// [ ] upsert_map
//     - insert
//     - update filename, exchangeid
//     - update does not set NULL exchangeid
//     => assert with .map, .maps, .map_files, .playlist
// [ ] playlist_add
// [ ] playlist_remove
// [x] nb_records
// [ ] top_record
// [ ] top_records
// [ ] player_record
// [x] player_records
// [ ] nb_players_with_record
// [x] maps_without_player_record
// [x] record_preview
// [x] upsert_record
// [ ] player_preferences
// [ ] count_map_preferences
// [ ] upsert_preference
// [ ] map_rankings
// [ ] delete_player
// [ ] delete_map
// [ ] delete_old_ghosts

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

#[tokio::test]
async fn test_history_played_none() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;

    let mut expected = vec![
        History {
            player_login: "login1".to_string(),
            map_uid: "uid1".to_string(),
            last_played: None,
            nb_maps_since: 0,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid2".to_string(),
            last_played: None,
            nb_maps_since: 0,
        },
    ];

    let mut actual = db.history("login1").await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_played_some() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let map3 = map_evidence("uid3", "file3");
    let map4 = map_evidence("uid4", "file4");
    let map1_last_played = now().sub(Duration::seconds(1));
    let map2_last_played = now();

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_map(&map3).await?;
    db.upsert_map(&map4).await?;

    db.add_history(&player1.login, &map1.metadata.uid, &map1_last_played)
        .await?;
    db.add_history(&player1.login, &map2.metadata.uid, &map2_last_played)
        .await?;

    let mut expected = vec![
        History {
            player_login: "login1".to_string(),
            map_uid: "uid1".to_string(),
            last_played: Some(map1_last_played),
            nb_maps_since: 1,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid2".to_string(),
            last_played: Some(map2_last_played),
            nb_maps_since: 0,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid3".to_string(),
            last_played: None,
            nb_maps_since: 2,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid4".to_string(),
            last_played: None,
            nb_maps_since: 2,
        },
    ];

    let mut actual = db.history("login1").await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_played_all() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let map3 = map_evidence("uid3", "file3");
    let map1_last_played = now().sub(Duration::seconds(2));
    let map2_last_played = now().sub(Duration::seconds(1));
    let map3_last_played = now();

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_map(&map3).await?;

    db.add_history(&player1.login, &map1.metadata.uid, &map1_last_played)
        .await?;
    db.add_history(&player1.login, &map2.metadata.uid, &map2_last_played)
        .await?;
    db.add_history(&player1.login, &map3.metadata.uid, &map3_last_played)
        .await?;

    let mut expected = vec![
        History {
            player_login: "login1".to_string(),
            map_uid: "uid1".to_string(),
            last_played: Some(map1_last_played),
            nb_maps_since: 2,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid2".to_string(),
            last_played: Some(map2_last_played),
            nb_maps_since: 1,
        },
        History {
            player_login: "login1".to_string(),
            map_uid: "uid3".to_string(),
            last_played: Some(map3_last_played),
            nb_maps_since: 0,
        },
    ];

    let mut actual = db.history("login1").await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_update() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map_evidence("uid1", "file1");
    let map1_last_played = now().sub(Duration::seconds(1));

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1).await?;

    db.add_history(&player1.login, &map1.metadata.uid, &map1_last_played)
        .await?;
    let map1_last_played = now();
    db.add_history(&player1.login, &map1.metadata.uid, &map1_last_played)
        .await?;

    let mut expected = vec![History {
        player_login: "login1".to_string(),
        map_uid: "uid1".to_string(),
        last_played: Some(map1_last_played),
        nb_maps_since: 0,
    }];

    let mut actual = db.history("login1").await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_maps_without_player_record() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_record(&rec1).await?;

    let expected = vec!["uid2".to_string()];
    let actual = db.maps_without_player_record("login1").await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_record_preview_first() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map_evidence("uid1", "file1");
    let rec1 = record_evidence("login1", "uid1", 10000);

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1).await?;

    let actual = db.record_preview(&rec1).await?;
    assert_eq!(1, actual);

    Ok(())
}

#[tokio::test]
async fn test_record_preview_top() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid1", 9000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_record(&rec1).await?;

    let actual = db.record_preview(&rec2).await?;
    assert_eq!(1, actual);

    Ok(())
}

#[tokio::test]
async fn test_record_preview() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map_evidence("uid1", "file1");
    let map2 = map_evidence("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid1", 11000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1).await?;
    db.upsert_map(&map2).await?;
    db.upsert_record(&rec1).await?;

    let actual = db.record_preview(&rec2).await?;
    assert_eq!(2, actual);

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
            author_millis: 0,
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
