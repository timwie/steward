use std::cmp::max;
use std::collections::HashMap;
use std::sync::Arc;

use indexmap::map::IndexMap;
use tokio::sync::{RwLock, RwLockReadGuard};

use async_trait::async_trait;

use crate::config::MAX_DISPLAYED_SERVER_RANKS;
use crate::controller::LivePlayers;
use crate::database::Database;
use crate::event::{ServerRankDiff, ServerRankingDiff};
use std::borrow::Cow;

/// Use to lookup the current server rankings.
/// They are updated after every race.
#[async_trait]
pub trait LiveServerRanking: Send + Sync {
    /// While holding this guard, the state is read-only, and can be referenced.
    async fn lock(&self) -> RwLockReadGuard<'_, ServerRankingState>;

    /// The number of players that have a server rank.
    async fn max_pos(&self) -> usize {
        self.lock().await.max_pos()
    }
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

pub struct ServerRank {
    pub pos: usize,
    pub player_login: String,
    pub player_nick_name: String,

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
/// Players will earn a "win" on each map, for every player
/// that has a worse personal best (or none at all).
///
/// For example, if a player has the 50th rank on a map, and the
/// server has had 200 players (with at least one record on any map) in total,
/// they get `199 max wins - 49 losses = 150 wins` for that map. How many of
/// those 200 players have actually set a record on that map is irrelevant.
async fn calc_server_ranking(db: &Arc<dyn Database>) -> IndexMap<Cow<'static, str>, ServerRank> {
    // This is a lazy way of calculating the server ranking,
    // which will look at the entire data set of records every time.
    // A more performant solution could compare the records of the current map
    // at the start, and the records at the end of a race. The differences
    // should be enough to update the server rankings (as long as records
    // are the only metric used).

    let max_wins = {
        let nb_players_with_record = db
            .nb_players_with_record()
            .await
            .expect("failed to load amount of players with at least one record")
            as usize;

        let max_wins_per_map = max(1, nb_players_with_record) - 1 as usize;

        let nb_maps = db.nb_maps().await.expect("failed to load amount of maps") as usize;

        nb_maps * max_wins_per_map
    };

    let map_ranks = db
        .map_rankings()
        .await
        .expect("failed to load map rankings");

    let mut losses = IndexMap::<&str, usize>::new(); // player login -> nb of losses
    let mut nick_names = HashMap::<&str, String>::new(); // player login -> nick name

    for map_rank in map_ranks.iter() {
        *losses.entry(&map_rank.player_login).or_insert(0) += map_rank.max_pos as usize - 1;
        nick_names.insert(&map_rank.player_login, map_rank.player_nick_name.clone());
    }

    // Less losses is better
    losses.sort_by(|_, a_losses, _, b_losses| a_losses.cmp(b_losses));

    losses
        .into_iter()
        .enumerate()
        .map(|(idx, (login, nb_losses))| {
            let rank = ServerRank {
                pos: idx + 1,
                player_login: login.to_string(),
                player_nick_name: nick_names.remove(login).unwrap(),
                nb_wins: max_wins - nb_losses,
                nb_losses,
            };
            (login.to_string().into(), rank)
        })
        .collect()
}

#[derive(Clone)]
pub struct ServerRankController {
    state: Arc<RwLock<ServerRankingState>>,
    db: Arc<dyn Database>,
    live_players: Arc<dyn LivePlayers>,
}

impl ServerRankController {
    pub async fn init(db: &Arc<dyn Database>, live_players: &Arc<dyn LivePlayers>) -> Self {
        let state = ServerRankingState {
            all_ranks: calc_server_ranking(db).await,
        };
        ServerRankController {
            state: Arc::new(RwLock::new(state)),
            db: db.clone(),
            live_players: live_players.clone(),
        }
    }

    /// Update the server ranking, and return information of changed
    /// ranks for connected players.
    pub async fn update(&self) -> ServerRankingDiff {
        let mut state = self.state.write().await;
        let live_players = self.live_players.lock().await;

        // Remove all rankings of offline players, as they don't need a diff.
        state
            .all_ranks
            .retain(|login, _| live_players.uid(&login).is_some());

        // Calculate new ranking from scratch
        let new_ranking = calc_server_ranking(&self.db).await;

        // List for newly ranked players
        let first_ranks: Vec<(i32, &ServerRank)> = live_players
            .info_all()
            .into_iter()
            .map(|info| (Cow::<'_, str>::from(&info.login), info))
            .filter(|(key, _)| !state.all_ranks.contains_key(key))
            .filter_map(|(key, info)| new_ranking.get(&key).map(|r| (info.uid, r)))
            .collect();

        let mut diffs: HashMap<i32, ServerRankDiff> = state
            .all_ranks
            .iter()
            .filter_map(|(key, old_rank)| match live_players.uid(&key) {
                None => None,
                Some(uid) => {
                    let new_rank = new_ranking.get(key).unwrap();
                    let diff = ServerRankDiff {
                        player_nick_name: old_rank.player_nick_name.clone(),
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
                    player_nick_name: rank.player_nick_name.clone(),
                    new_pos: rank.pos,
                    gained_pos: new_ranking.len() as i32 - rank.pos as i32,
                    gained_wins: rank.nb_wins as i32,
                },
            );
        }

        // Overwrite old ranking.
        state.all_ranks = new_ranking;

        ServerRankingDiff {
            diffs,
            max_pos: state.all_ranks.len(),
        }
    }
}

#[async_trait]
impl LiveServerRanking for ServerRankController {
    async fn lock(&self) -> RwLockReadGuard<'_, ServerRankingState> {
        self.state.read().await
    }
}
