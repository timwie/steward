use std::ops::Sub;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime, SubsecRound, Utc};
use testcontainers::*;

use steward::database::*;
use steward::server::{DisplayString, PlayerInfo, TeamId};

// TODO add database tests
// [x] migrate
// [x] player
// [x] upsert_player
// [x] add_history
// [ ] history
//     - test with empty/non-empty map_uids
// [ ] map_file
// [ ] maps
//     - test with empty/non-empty map_uids
// [ ] map
// [ ] upsert_map
//     - insert
//     - update filename, exchangeid
//     - update does not set NULL exchangeid
//     => assert with .map, .maps, .map_files
// [ ] nb_records
//     - test with different numbers of laps
// [ ] top_record
// [ ] top_records
// [ ] player_record
//     - test with different numbers of laps
// [ ] records
//     - test with limit
//     - test with different numbers of laps
//     - test with empty/non-empty map_uids
// [ ] nb_players_with_record
// [x] maps_without_player_record
// [ ] record_preview
//     - test with different numbers of laps
// [ ] upsert_record
//     - test with different numbers of laps
// [ ] player_preferences
// [ ] count_map_preferences
// [ ] upsert_preference
// [ ] map_rankings
//     - test with empty/non-empty map_uids
// [ ] delete_player
// [ ] delete_map

/// Spins up a Postgres database in a Docker container.
async fn clean_db() -> Result<DatabaseClient> {
    // Enable logging output
    let _ = env_logger::builder().is_test(true).try_init();

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

    let docker = clients::Cli::default();

    log::info!("starting container...");
    let container = docker.run(generic_postgres);
    log::info!("container started");

    let pg_conn_str = format!(
        "postgres://{}:{}@localhost:{}/{}",
        user,
        password,
        container
            .get_host_port(5432)
            .expect("failed to determine Postgres host port"),
        db
    );

    log::info!("connecting to container database...");
    let client = pg_connect(&pg_conn_str, std::time::Duration::from_secs(5))
        .await
        .expect("postgres not running");
    log::info!("connected to container database");

    log::info!("clear database...");
    client.clear().await?;

    log::info!("migrate database...");
    client.migrate().await?;

    log::info!("completed test setup");

    // TODO when using testcontainers 0.11, trying to get a connection from the pool
    //  results in 'Error: Timed out in bb8'

    Ok(client)
}

#[tokio::test]
async fn test_player_insert() -> Result<()> {
    let db = clean_db().await?;
    let expected_info = player_info("login", "nickname");
    let expected = Player {
        login: "login".to_string(),
        display_name: DisplayString::from("nickname".to_string()),
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
        display_name: DisplayString::from("new nickname".to_string()),
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
    assert_eq!(0, db.nb_records("uid1", 0).await?);

    let map = map("uid1", "file1");
    db.upsert_map(&map, vec![]).await?;
    assert_eq!(0, db.nb_records("uid1", 0).await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_one() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map, vec![]).await?;
    db.upsert_record(&rec).await?;

    assert_eq!(1, db.nb_records("uid1", 0).await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_multiple_maps() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let rec11 = record_evidence("login1", "uid1", 10000);
    let rec12 = record_evidence("login2", "uid1", 10000);
    let rec21 = record_evidence("login1", "uid2", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
    db.upsert_record(&rec11).await?;
    db.upsert_record(&rec12).await?;
    db.upsert_record(&rec21).await?;

    assert_eq!(2, db.nb_records("uid1", 0).await?);
    assert_eq!(1, db.nb_records("uid2", 0).await?);

    Ok(())
}

#[tokio::test]
async fn test_nb_records_one_per_player() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map("uid1", "file1");
    let rec1 = record_evidence("login", "uid1", 10000);
    let rec2 = record_evidence("login", "uid1", 9000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map, vec![]).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    assert_eq!(1, db.nb_records("uid1", 0).await?);

    Ok(())
}

#[tokio::test]
async fn test_player_record_some() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map, vec![]).await?;
    db.upsert_record(&rec).await?;

    let expected = record(1, "nickname", rec);
    let expected = Some(expected);

    let actual = db.player_record("uid1", "login", 0).await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_records_single() -> Result<()> {
    let db = clean_db().await?;

    let player = player_info("login", "nickname");
    let map = map("uid1", "file1");
    let rec = record_evidence("login", "uid1", 10000);
    db.upsert_player(&player).await?;
    db.upsert_map(&map, vec![]).await?;
    db.upsert_record(&rec).await?;

    let expected = record(1, "nickname", rec);
    let expected = vec![expected];

    let actual = db.records(vec!["uid1"], vec!["login"], 0, None).await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_records_multiple_players() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map = map("uid1", "file1");
    let rec1 = record_evidence("login1", "uid1", 9000);
    let rec2 = record_evidence("login2", "uid1", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map, vec![]).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    let expected1 = record(1, "nickname1", rec1);
    let expected2 = record(2, "nickname2", rec2);
    let expected = vec![expected1, expected2];

    let actual = db
        .records(vec!["uid1"], vec!["login1", "login2"], 0, None)
        .await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_records_multiple_maps() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid2", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
    db.upsert_record(&rec1).await?;
    db.upsert_record(&rec2).await?;

    let expected = record(1, "nickname1", rec1);
    let expected = vec![expected];

    let actual = db
        .records(vec!["uid1"], vec!["login1", "login2"], 0, None)
        .await?;
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_played_none() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;

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

    let mut actual = db.history("login1", vec!["uid1", "uid2"]).await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_played_some() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let map3 = map("uid3", "file3");
    let map4 = map("uid4", "file4");
    let map1_last_played = now().sub(Duration::seconds(1));
    let map2_last_played = now();

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
    db.upsert_map(&map3, vec![]).await?;
    db.upsert_map(&map4, vec![]).await?;

    db.add_history(&player1.login, &map1.uid, &map1_last_played)
        .await?;
    db.add_history(&player1.login, &map2.uid, &map2_last_played)
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

    let mut actual = db
        .history("login1", vec!["uid1", "uid2", "uid3", "uid4"])
        .await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_played_all() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let map3 = map("uid3", "file3");
    let map1_last_played = now().sub(Duration::seconds(2));
    let map2_last_played = now().sub(Duration::seconds(1));
    let map3_last_played = now();

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
    db.upsert_map(&map3, vec![]).await?;

    db.add_history(&player1.login, &map1.uid, &map1_last_played)
        .await?;
    db.add_history(&player1.login, &map2.uid, &map2_last_played)
        .await?;
    db.add_history(&player1.login, &map3.uid, &map3_last_played)
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

    let mut actual = db.history("login1", vec!["uid1", "uid2", "uid3"]).await?;

    expected.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    actual.sort_by(|a, b| a.map_uid.cmp(&b.map_uid));
    assert_eq!(expected, actual);

    Ok(())
}

#[tokio::test]
async fn test_history_update() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let map1 = map("uid1", "file1");
    let map1_last_played = now().sub(Duration::seconds(1));

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1, vec![]).await?;

    db.add_history(&player1.login, &map1.uid, &map1_last_played)
        .await?;
    let map1_last_played = now();
    db.add_history(&player1.login, &map1.uid, &map1_last_played)
        .await?;

    let mut expected = vec![History {
        player_login: "login1".to_string(),
        map_uid: "uid1".to_string(),
        last_played: Some(map1_last_played),
        nb_maps_since: 0,
    }];

    let mut actual = db.history("login1", vec!["uid1"]).await?;

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
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
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
    let map1 = map("uid1", "file1");
    let rec1 = record_evidence("login1", "uid1", 10000);

    db.upsert_player(&player1).await?;
    db.upsert_map(&map1, vec![]).await?;

    let actual = db.record_preview(&rec1).await?;
    assert_eq!(1, actual);

    Ok(())
}

#[tokio::test]
async fn test_record_preview_top() -> Result<()> {
    let db = clean_db().await?;

    let player1 = player_info("login1", "nickname1");
    let player2 = player_info("login2", "nickname2");
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid1", 9000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
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
    let map1 = map("uid1", "file1");
    let map2 = map("uid2", "file2");
    let rec1 = record_evidence("login1", "uid1", 10000);
    let rec2 = record_evidence("login2", "uid1", 11000);
    db.upsert_player(&player1).await?;
    db.upsert_player(&player2).await?;
    db.upsert_map(&map1, vec![]).await?;
    db.upsert_map(&map2, vec![]).await?;
    db.upsert_record(&rec1).await?;

    let actual = db.record_preview(&rec2).await?;
    assert_eq!(2, actual);

    Ok(())
}

fn player_info(login: &str, display_name: &str) -> PlayerInfo {
    PlayerInfo {
        uid: 0,
        login: login.to_string(),
        display_name: DisplayString::from(display_name.to_string()),
        flag_digit_mask: 101_000_000,
        spectator_digit_mask: 2_551_010,
        team_id: Some(TeamId::Blue),
    }
}

fn map(uid: &str, file_name: &str) -> Map {
    Map {
        uid: uid.to_string(),
        file_name: file_name.to_string(),
        name: DisplayString::from("".to_string()),
        author_login: "".to_string(),
        author_display_name: DisplayString::from("".to_string()),
        author_millis: 0,
        added_since: now(),
        exchange_id: None,
    }
}

fn record_evidence(login: &str, map_uid: &str, millis: i32) -> RecordEvidence {
    RecordEvidence {
        player_login: login.to_string(),
        map_uid: map_uid.to_string(),
        millis,
        timestamp: now(),
        nb_laps: 0,
    }
}

fn record(pos: i64, display_name: &str, ev: RecordEvidence) -> Record {
    Record {
        map_uid: ev.map_uid,
        map_rank: pos,
        player_login: ev.player_login,
        player_display_name: DisplayString::from(display_name.to_string()),
        millis: ev.millis,
        timestamp: ev.timestamp,
        nb_laps: 0,
    }
}

fn now() -> NaiveDateTime {
    // If we want to test date equality, we need to round at least a few nano
    // digits, since we lose some precision when storing in the database, f.e.
    //      before: 2020-06-06T19:49:59.973170303
    //       after: 2020-06-06T19:49:59.973170
    Utc::now().naive_utc().round_subsecs(5)
}
