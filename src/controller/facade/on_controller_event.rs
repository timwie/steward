use async_recursion::async_recursion;

use crate::chat::{ServerMessage, TopRankMessage};
use crate::config::{
    MAX_ANNOUNCED_RANK, MAX_ANNOUNCED_RECORD, MAX_ANNOUNCED_RECORD_IMPROVEMENT,
    MAX_NB_ANNOUNCED_RANKS,
};
use crate::controller::{Controller, LiveConfig, LivePlayers, LivePlaylist, LiveQueue};
use crate::event::{
    Command, ConfigDiff, ControllerEvent, PbDiff, PlayerTransition, PlaylistDiff, ServerRankingDiff,
};

impl Controller {
    #[async_recursion]
    pub(super) async fn on_controller_event(&self, event: ControllerEvent<'async_recursion>) {
        use ControllerEvent::*;

        log::debug!("{:#?}", &event);

        if let Some(server_msg) = self.message_from_event(&event).await {
            self.chat.announce(server_msg).await;
        }

        match event {
            BeginRun { player_login } => {
                self.records.reset_run(&player_login).await;
                self.widget.end_run_outro_for(&player_login).await;
            }

            BeginMap(_) => {}

            BeginIntro => {
                self.race.reset().await;

                self.schedule.set_time_limit().await;

                self.widget.begin_intro().await;
            }

            EndIntro { player_login } => {
                self.widget.end_intro_for(&player_login).await;
            }

            EndRun(pb_diff) => {
                self.widget.begin_run_outro_for(&pb_diff).await;
                self.widget.refresh_personal_best(&pb_diff).await;

                if let Some(map_uid) = &self.playlist.current_map_uid().await {
                    self.prefs.update_history(pb_diff.player_uid, map_uid).await;
                }
            }

            BeginOutro => {
                self.widget.begin_outro_and_vote().await;
                let _ = self.race.reset().await;
            }

            EndOutro => {
                self.widget.end_outro().await;
            }

            EndMap => {
                // Update the current map
                let next_index = self.server.playlist_next_index().await;
                self.playlist.set_index(next_index).await;

                // Re-sort the queue: the current map will move to the back.
                if let Some(diff) = self.queue.sort_queue().await {
                    let ev = NewQueue(diff);
                    self.on_controller_event(ev).await;
                }
            }

            EndVote => {
                self.prefs.reset_restart_votes().await;

                let queue_preview = self.queue.peek().await;
                self.widget.end_vote(queue_preview).await;

                let next_map = self.queue.pop_front().await;
                self.records.load_for_map(&next_map).await;
            }

            NewQueue(diff) => {
                self.widget.refresh_queue_and_schedule(&diff).await;
            }

            NewPlayerList(diff) => {
                self.records.update_for_player(&diff).await;
                self.prefs.update_for_player(&diff).await;
                self.widget.refresh_for_player(&diff).await;
            }

            NewPlaylist(playlist_diff) => {
                // Update active preferences. This has to happen before re-sorting the queue.
                self.prefs.update_for_map(&playlist_diff).await;

                // Re-sort the map queue.
                let queue_diff = self.queue.insert_or_remove(&playlist_diff).await;
                let ev = NewQueue(queue_diff);
                self.on_controller_event(ev).await;

                // Add or remove the map from the schedule.
                self.schedule.insert_or_remove(&playlist_diff).await;

                // Update playlist UI.
                self.widget.refresh_playlist().await;

                // At this point, we could update the server ranking, since adding &
                // removing maps will affect it. But, doing so would give us weird
                // server ranking diffs during the outro. The diffs are only meaningful
                // if we calculate the ranking once per map.
            }

            NewServerRanking(change) => {
                self.widget.refresh_server_ranking(&change).await;
            }

            IssueCommand(Command::Player { from, cmd }) => self.on_cmd(&from, cmd).await,

            IssueCommand(Command::Admin { from, cmd }) => self.on_admin_cmd(&from, cmd).await,

            IssueCommand(Command::SuperAdmin { from, cmd }) => {
                self.on_super_admin_cmd(&from, cmd).await
            }

            IssueAction { from_login, action } => {
                if let Some(info) = self.players.info(&from_login).await {
                    self.on_action(&info, action).await;
                }
            }

            NewConfig { change, from_login } => {
                self.on_config_change(from_login, change).await;
            }
        }
    }

    async fn on_config_change(&self, from_login: &str, diff: ConfigDiff) {
        use ConfigDiff::*;

        let from_nick_name = match self.players.nick_name(from_login).await {
            Some(name) => name,
            None => return,
        };

        match diff {
            NewTimeLimit { .. } => {
                self.schedule.set_time_limit().await;
                self.widget.refresh_schedule().await;

                self.chat
                    .announce(ServerMessage::TimeLimitChanged {
                        admin_name: &from_nick_name.formatted,
                    })
                    .await;
            }
            NewOutroDuration { .. } => {
                self.widget.refresh_schedule().await;
            }
        }
    }

    #[allow(clippy::needless_lifetimes)]
    async fn message_from_event<'a>(
        &self,
        event: &'a ControllerEvent<'_>,
    ) -> Option<ServerMessage<'a>> {
        use ControllerEvent::*;
        use ServerMessage::*;

        match event {
            NewPlayerList(diff) => {
                use PlayerTransition::*;

                match diff.transition {
                    AddPlayer | AddSpectator | AddPureSpectator => Some(Joining {
                        nick_name: &diff.info.nick_name.formatted,
                    }),
                    RemovePlayer | RemoveSpectator | RemovePureSpectator => Some(Leaving {
                        nick_name: &diff.info.nick_name.formatted,
                    }),
                    _ => None,
                }
            }

            BeginMap(loaded_map) => Some(CurrentMap {
                name: &loaded_map.name.formatted,
                author: &loaded_map.author_login,
            }),

            BeginOutro => {
                let vote_duration = self.config.vote_duration().await;
                let min_restart_vote_ratio = self.queue.lock().await.min_restart_vote_ratio;
                Some(VoteNow {
                    duration: vote_duration,
                    threshold: min_restart_vote_ratio,
                })
            }

            EndRun(PbDiff {
                new_pos,
                pos_gained,
                new_record: Some(new_record),
                ..
            }) if *pos_gained > 0 && *new_pos <= MAX_ANNOUNCED_RECORD => Some(TopRecord {
                player_nick_name: &new_record.player_nick_name.formatted,
                new_map_rank: *new_pos,
                millis: new_record.millis as usize,
            }),

            EndRun(PbDiff {
                new_pos,
                pos_gained,
                new_record: Some(new_record),
                millis_diff: Some(diff),
                ..
            }) if *pos_gained == 0 && *diff < 0 && *new_pos <= MAX_ANNOUNCED_RECORD_IMPROVEMENT => {
                Some(TopRecordImproved {
                    player_nick_name: &new_record.player_nick_name.formatted,
                    map_rank: *new_pos,
                    millis: new_record.millis as usize,
                })
            }

            NewPlaylist(PlaylistDiff::AppendNew(map)) => Some(NewMap {
                name: &map.name.formatted,
                author: &map.author_login,
            }),

            NewPlaylist(PlaylistDiff::Append(map)) => Some(AddedMap {
                name: &map.name.formatted,
            }),

            NewPlaylist(PlaylistDiff::Remove { map, .. }) => Some(RemovedMap {
                name: &map.name.formatted,
            }),

            NewServerRanking(ServerRankingDiff { diffs, .. }) => {
                let mut top_ranks: Vec<TopRankMessage> = diffs
                    .values()
                    .filter_map(|diff| {
                        if diff.gained_pos > 0 && diff.new_pos <= MAX_ANNOUNCED_RANK {
                            Some(TopRankMessage {
                                nick_name: &diff.player_nick_name.formatted,
                                rank: diff.new_pos,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                top_ranks.sort_by_key(|tr| tr.rank); // lowest ranks (highest number) last
                top_ranks = top_ranks.into_iter().take(MAX_NB_ANNOUNCED_RANKS).collect();
                top_ranks.reverse(); // highest ranks last -> more prominent in chat
                Some(NewTopRanks(top_ranks))
            }

            _ => None,
        }
    }
}
