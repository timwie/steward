use std::borrow::Cow;
use std::cmp::max;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use async_trait::async_trait;
use indexmap::map::IndexMap;
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::constants::MAX_DISPLAYED_SERVER_RANKS;
use crate::controller::LivePlayers;
use crate::database::timeattack::TimeAttackQueries;
use crate::database::{DatabaseClient, RecordQueries};
use crate::event::{ServerRankDiff, ServerRankingDiff};
use crate::server::{Calls, DisplayString, Server};

/// Use to lookup the current server rankings.
/// They are updated after every race.
#[async_trait]
pub trait LiveServerRanking: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, ServerRankingState>;
}

pub struct ServerRankingState {
    /// A collection of server ranks, sorted from best to worst.
    all_ranks: IndexMap<Cow<'static, str>, ServerRank>,
}

impl ServerRankingState {
    /// Returns a number of top server ranks.
    /// The number is determined by how many ranks we want
    /// to display in-game. The list is sorted from better
    /// to worse.
    pub fn top_ranks(&self) -> impl Iterator<Item = &ServerRank> {
        self.all_ranks.values().take(MAX_DISPLAYED_SERVER_RANKS)
    }

    /// Returns a the server rank of the specified player, or `None`
    /// if they don't have a rank yet.
    pub fn rank_of<'a>(&'a self, player_login: &'a str) -> Option<&'a ServerRank> {
        let key: Cow<'a, str> = player_login.into();
        self.all_ranks.get(&key)
    }

    /// The number of players that have a server rank.
    pub fn max_pos(&self) -> usize {
        self.all_ranks.len()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ServerRank {
    pub pos: usize,
    pub player_login: String,
    pub player_display_name: DisplayString,

    /// The number of beaten records summed for every map.
    pub nb_wins: usize,

    /// The number of records better than this player's summed for every map.
    pub nb_losses: usize,
}

/// Ranks all players that have set at least one record on this server.
///
/// Returns a collection that
/// - maps a player's login to their server rank,
/// - and is iterated from rank 1 to the last in order.
///
/// Players will earn a "win" on each map in the playlist, for every player
/// that has a worse personal best (or none at all). Maps that are not
/// in the playlist should not count, since new players cannot set records
/// on them, making it hard for them to catch up to other players.
///
/// For example, if a player has the 50th rank on a map, and the
/// server has had 200 players (with at least one record on any map) in total,
/// they get `199 max wins - 49 losses = 150 wins` for that map. How many of
/// those 200 players have actually set a record on that map is irrelevant.
async fn calc_server_ranking(
    db: &DatabaseClient,
    map_uids: Vec<&str>,
) -> IndexMap<Cow<'static, str>, ServerRank> {
    // This is a lazy way of calculating the server ranking,
    // which will look at the entire data set of records every time.
    // A more performant solution could compare the records of the current map
    // at the start, and the records at the end of a race. The differences
    // should be enough to update the server rankings (as long as records
    // are the only metric used).

    // Every player with at least one record will be ranked.
    let nb_ranked_players =
        db.nb_players_with_record()
            .await
            .expect("failed to load amount of players with at least one record") as usize;

    // You can beat (nb_ranked_players - 1) players on every map.
    let max_total_wins = {
        let max_wins_per_map: usize = max(1, nb_ranked_players) - 1;
        map_uids.len() * max_wins_per_map
    };

    // Using the map rankings, we can count the number of wins for each player.
    // Note that we cannot count the losses, since you also gain losses by not having
    // a map rank at all.
    let map_ranks = db
        .map_rankings(map_uids)
        .await
        .expect("failed to load map rankings");

    let mut nb_wins = IndexMap::<&str, usize>::new(); // player login -> nb of wins
    let mut display_names = HashMap::<&str, DisplayString>::new(); // player login -> display name

    for map_rank in map_ranks.iter() {
        let nb_map_wins = nb_ranked_players - map_rank.pos as usize;
        *nb_wins.entry(&map_rank.player_login).or_insert(0) += nb_map_wins;

        if !display_names.contains_key(map_rank.player_login.as_str()) {
            display_names.insert(&map_rank.player_login, map_rank.player_display_name.clone());
        }
    }

    // More wins is better: put them first.
    nb_wins.sort_by(|_, a_wins, _, b_wins| b_wins.cmp(a_wins));

    nb_wins
        .into_iter()
        .enumerate()
        .map(|(idx, (login, nb_wins))| {
            let rank = ServerRank {
                pos: idx + 1,
                player_login: login.to_string(),
                player_display_name: display_names.remove(login).unwrap(),
                nb_wins,
                nb_losses: max_total_wins - nb_wins,
            };
            (login.to_string().into(), rank)
        })
        .collect()
}

#[derive(Clone)]
pub struct ServerRankController {
    state: Arc<RwLock<ServerRankingState>>,
    server: Server,
    db: DatabaseClient,
    live_players: Arc<dyn LivePlayers>,
}

impl ServerRankController {
    pub async fn init(
        server: &Server,
        db: &DatabaseClient,
        live_players: &Arc<dyn LivePlayers>,
    ) -> Self {
        let playlist = server.playlist().await;
        let playlist_uids = playlist.iter().map(|m| m.uid.deref()).collect();

        let state = ServerRankingState {
            all_ranks: calc_server_ranking(db, playlist_uids).await,
        };
        ServerRankController {
            state: Arc::new(RwLock::new(state)),
            server: server.clone(),
            db: db.clone(),
            live_players: live_players.clone(),
        }
    }

    /// Update the server ranking, and return information of changed
    /// ranks for connected players.
    pub async fn update(&self) -> ServerRankingDiff {
        let mut server_ranking_state = self.state.write().await;
        let players_state = self.live_players.lock().await;

        // Remove all rankings of offline players, as they don't need a diff.
        server_ranking_state
            .all_ranks
            .retain(|login, _| players_state.uid(&login).is_some());

        // Calculate new ranking from scratch
        let playlist = self.server.playlist().await;
        let playlist_uids = playlist.iter().map(|m| m.uid.deref()).collect();
        let new_ranking = calc_server_ranking(&self.db, playlist_uids).await;

        // List for newly ranked players
        let first_ranks: Vec<(i32, &ServerRank)> = players_state
            .info_all()
            .into_iter()
            .map(|info| (Cow::<'_, str>::from(&info.login), info))
            .filter(|(key, _)| !server_ranking_state.all_ranks.contains_key(key))
            .filter_map(|(key, info)| new_ranking.get(&key).map(|r| (info.uid, r)))
            .collect();

        let mut diffs: HashMap<i32, ServerRankDiff> = server_ranking_state
            .all_ranks
            .iter()
            .filter_map(|(key, old_rank)| match players_state.uid(&key) {
                None => None,
                Some(uid) => {
                    let new_rank = new_ranking.get(key).unwrap();
                    let diff = ServerRankDiff {
                        player_display_name: old_rank.player_display_name.clone(),
                        new_pos: new_rank.pos,
                        gained_pos: old_rank.pos as i32 - new_rank.pos as i32,
                        gained_wins: old_rank.nb_wins as i32 - new_rank.nb_wins as i32,
                    };
                    Some((*uid, diff))
                }
            })
            .collect();

        for (login, rank) in first_ranks {
            diffs.insert(
                login,
                ServerRankDiff {
                    player_display_name: rank.player_display_name.clone(),
                    new_pos: rank.pos,
                    gained_pos: new_ranking.len() as i32 - rank.pos as i32,
                    gained_wins: rank.nb_wins as i32,
                },
            );
        }

        // Overwrite old ranking.
        server_ranking_state.all_ranks = new_ranking;

        ServerRankingDiff {
            diffs,
            max_pos: server_ranking_state.all_ranks.len(),
        }
    }
}

#[async_trait]
impl LiveServerRanking for ServerRankController {
    async fn lock(&self) -> RwLockReadGuard<'_, ServerRankingState> {
        self.state.read().await
    }
}

#[cfg(feature = "unit_test")]
mod test {
    use std::default::Default;

    use super::*;

    #[tokio::test]
    async fn empty_server_ranking() {
        let mut mock_db: DatabaseClient = Default::default();
        let ranking = calc_server_ranking(&mock_db, vec![]).await;
        assert!(ranking.is_empty());

        mock_db.push_player("login1", "nick1");
        mock_db.push_map("uid1");
        mock_db.push_record("login1", "uid1", 10000);

        let ranking = calc_server_ranking(&mock_db, vec![]).await;
        assert!(ranking.is_empty());
    }

    #[tokio::test]
    async fn trivial_server_ranking() {
        let mut mock_db: DatabaseClient = Default::default();
        mock_db.push_player("login1", "nick1");
        mock_db.push_map("uid1");
        mock_db.push_record("login1", "uid1", 10000);

        let ranking = calc_server_ranking(&mock_db, vec!["uid1"]).await;
        assert_eq!(1, ranking.len());

        let actual = ranking.values().next().unwrap();
        let expected = ServerRank {
            pos: 1,
            player_login: "login1".to_string(),
            player_display_name: DisplayString::from("nick1".to_string()),
            nb_wins: 0,
            nb_losses: 0,
        };
        assert_eq!(actual, &expected);
    }

    #[tokio::test]
    async fn single_map_server_ranking() {
        let mut mock_db: DatabaseClient = Default::default();
        mock_db.push_player("login1", "nick1");
        mock_db.push_player("login2", "nick2");
        mock_db.push_player("login3", "nick3");
        mock_db.push_map("uid1");
        mock_db.push_record("login1", "uid1", 10000);
        mock_db.push_record("login2", "uid1", 20000);
        mock_db.push_record("login3", "uid1", 30000);

        let ranking = calc_server_ranking(&mock_db, vec!["uid1"]).await;

        let actual = ranking.values().next().unwrap();
        let expected = ServerRank {
            pos: 1,
            player_login: "login1".to_string(),
            player_display_name: DisplayString::from("nick1".to_string()),
            nb_wins: 2,
            nb_losses: 0,
        };
        assert_eq!(actual, &expected);

        let actual = ranking.values().nth(1).unwrap();
        let expected = ServerRank {
            pos: 2,
            player_login: "login2".to_string(),
            player_display_name: DisplayString::from("nick2".to_string()),
            nb_wins: 1,
            nb_losses: 1,
        };
        assert_eq!(actual, &expected);

        let actual = ranking.values().nth(2).unwrap();
        let expected = ServerRank {
            pos: 3,
            player_login: "login3".to_string(),
            player_display_name: DisplayString::from("nick3".to_string()),
            nb_wins: 0,
            nb_losses: 2,
        };
        assert_eq!(actual, &expected);
    }

    #[tokio::test]
    async fn multi_map_server_ranking() {
        let mut mock_db: DatabaseClient = Default::default();
        mock_db.push_player("login1", "nick1");
        mock_db.push_player("login2", "nick2");
        mock_db.push_map("uid1");
        mock_db.push_map("uid2");
        mock_db.push_map("uid3");
        mock_db.push_record("login1", "uid1", 10000);
        mock_db.push_record("login2", "uid1", 20000);
        mock_db.push_record("login1", "uid2", 10000);
        mock_db.push_record("login2", "uid2", 20000);
        mock_db.push_record("login1", "uid3", 20000);
        mock_db.push_record("login2", "uid3", 10000);

        let ranking = calc_server_ranking(&mock_db, vec!["uid1", "uid2", "uid3"]).await;

        let actual = ranking.values().next().unwrap();
        let expected = ServerRank {
            pos: 1,
            player_login: "login1".to_string(),
            player_display_name: DisplayString::from("nick1".to_string()),
            nb_wins: 2,
            nb_losses: 1,
        };
        assert_eq!(actual, &expected);

        let actual = ranking.values().nth(1).unwrap();
        let expected = ServerRank {
            pos: 2,
            player_login: "login2".to_string(),
            player_display_name: DisplayString::from("nick2".to_string()),
            nb_wins: 1,
            nb_losses: 2,
        };
        assert_eq!(actual, &expected);
    }

    #[tokio::test]
    async fn only_rank_playlist_maps() {
        let mut mock_db: DatabaseClient = Default::default();
        mock_db.push_player("login1", "nick1");
        mock_db.push_player("login2", "nick2");
        mock_db.push_map("uid1");
        mock_db.push_map("uid2");
        mock_db.push_record("login1", "uid1", 10000);
        mock_db.push_record("login2", "uid1", 20000);
        mock_db.push_record("login1", "uid2", 20000);
        mock_db.push_record("login2", "uid2", 10000);

        let ranking = calc_server_ranking(&mock_db, vec!["uid1"]).await;

        let actual = ranking.values().next().unwrap();
        let expected = ServerRank {
            pos: 1,
            player_login: "login1".to_string(),
            player_display_name: DisplayString::from("nick1".to_string()),
            nb_wins: 1,
            nb_losses: 0,
        };
        assert_eq!(actual, &expected);

        let actual = ranking.values().nth(1).unwrap();
        let expected = ServerRank {
            pos: 2,
            player_login: "login2".to_string(),
            player_display_name: DisplayString::from("nick2".to_string()),
            nb_wins: 0,
            nb_losses: 1,
        };
        assert_eq!(actual, &expected);
    }
}
