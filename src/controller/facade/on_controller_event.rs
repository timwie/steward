use async_recursion::async_recursion;

use crate::chat::{Command, CommandOutputResponse, CommandResponse, ServerMessage, TopRankMessage};
use crate::constants::{
    MAX_ANNOUNCED_RANK, MAX_ANNOUNCED_RECORD, MAX_ANNOUNCED_RECORD_IMPROVEMENT,
    MAX_NB_ANNOUNCED_RANKS,
};
use crate::controller::facade::announce;
use crate::controller::{Controller, LiveConfig, LivePlayers, LivePlaylist, LiveQueue};
use crate::event::{ControllerEvent, PbDiff, PlayerTransition, PlaylistDiff, ServerRankingDiff};
use crate::server::Calls;

impl Controller {
    #[async_recursion]
    pub(super) async fn on_controller_event(&self, event: ControllerEvent<'async_recursion>) {
        use ControllerEvent::*;

        log::debug!("{:#?}", &event);

        if let Some(server_msg) = self.message_from_event(&event).await {
            announce(&self.server, server_msg).await;
        }

        match event {
            BeginRun { player_login } => {
                // If this is the first time a player is at the start line,
                // their intro has just ended.
                let is_player_intro_end = self.race.add_contestant(&player_login).await;
                if is_player_intro_end {
                    let ev = ControllerEvent::EndIntro {
                        player_login: &player_login,
                    };
                    self.on_controller_event(ev).await;
                }

                self.widget.end_run_outro_for(&player_login).await;
            }

            ContinueRun(event) => {
                if !event.is_finish {
                    return;
                }

                self.race.update(&event).await;

                // Storing records involves file IO; run in separate task.
                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    if let Some(pb_diff) = controller.records.end_run(&event).await {
                        let ev = ControllerEvent::FinishRun(pb_diff);
                        controller.on_controller_event(ev).await;
                    }
                });
            }

            DesyncRun { .. } => {
                // TODO unsure if we have to do something here
                //  => as a precaution, we could not consider player runs until they give
                //     up their current run
            }

            BeginIntro => {
                self.widget.begin_intro().await;
            }

            EndIntro { player_login } => {
                self.widget.end_intro_for(&player_login).await;
            }

            FinishRun(pb_diff) => {
                self.widget.begin_run_outro_for(&pb_diff).await;
                self.widget.refresh_personal_best(&pb_diff).await;

                if let Some(map_uid) = &self.playlist.current_map_uid().await {
                    self.prefs.update_history(pb_diff.player_uid, map_uid).await;
                }
            }

            BeginOutro => {
                self.widget.begin_outro_and_vote().await;
                let _ = self.race.reset().await;

                // Spawn a task to re-calculate the server ranking,
                // which could be expensive, depending on how we do it.
                let controller = self.clone(); // 'self' with 'static lifetime
                let _ = tokio::spawn(async move {
                    let ranking_change = controller.ranking.update().await;
                    let new_ranking_ev = ControllerEvent::NewServerRanking(ranking_change);
                    controller.on_controller_event(new_ranking_ev).await;
                });

                let end_vote_ev = ControllerEvent::BeginVote;
                self.on_controller_event(end_vote_ev).await;
            }

            EndOutro => {
                self.race.reset().await;

                self.schedule.set_time_limit().await;

                self.widget.end_outro().await;
            }

            ChangeMap => {
                // Update the current map
                let new_playlist_index = self.server.playlist_next_index().await;
                let next_map = self.playlist.set_index(new_playlist_index).await;

                // Re-sort the queue: the current map will move to the back.
                if let Some(diff) = self.queue.sort_queue().await {
                    let ev = NewQueue(diff);
                    self.on_controller_event(ev).await;
                }

                // Load data for next map
                self.records.load_for_map(&next_map).await;
            }

            BeginVote => {
                // Delay for the duration of the vote.
                // Spawn a task to not block the callback loop.
                let controller = self.clone(); // 'self' with 'static lifetime
                let vote_duration = self.config.vote_duration().await;
                let _ = tokio::spawn(async move {
                    tokio::time::delay_for(
                        vote_duration.to_std().expect("failed to delay vote end"),
                    )
                    .await;
                    let end_vote_ev = ControllerEvent::EndVote;
                    controller.on_controller_event(end_vote_ev).await;
                });
            }

            EndVote => {
                // Sort the queue, now that all restart votes have been cast.
                // The next map is now at the top of the queue.
                if let Some(diff) = self.queue.sort_queue().await {
                    let ev = ControllerEvent::NewQueue(diff);
                    self.on_controller_event(ev).await;
                }

                let queue_preview = self.queue.peek().await;
                self.widget.end_vote(queue_preview).await;

                let next_map = self.queue.pop_front().await;

                self.prefs.reset_restart_votes().await;

                let msg = ServerMessage::NextMap {
                    name: &next_map.name.formatted,
                    author: &next_map.author_display_name.formatted,
                };
                announce(&self.server, msg).await;
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

            IssueCommand(ctxt, Command::Help) => {
                let msg = CommandResponse::Output(CommandOutputResponse::CommandReference(ctxt));
                self.widget.show_popup(msg, &ctxt.player.login).await;
            }

            IssueCommand(ctxt, Command::Player(cmd)) => self.on_cmd(&ctxt.player, cmd).await,

            IssueCommand(ctxt, Command::Admin(cmd)) => self.on_admin_cmd(&ctxt.player, cmd).await,

            IssueCommand(ctxt, Command::SuperAdmin(cmd)) => {
                self.on_super_admin_cmd(&ctxt.player, cmd).await
            }

            IssueAction { from_login, action } => {
                if let Some(info) = self.players.info(&from_login).await {
                    self.on_action(&info, action).await;
                }
            }

            NewConfig { change, from_login } => {
                self.on_config_change(from_login, change).await;
            }

            BeginWarmup => {}
            EndWarmup => {}
            BeginPause => {}
            EndPause => {}
            ChangeMode(_) => {} // TODO so far, we only handle the TimeAttack mode

            ChatMessage { from, message } => {
                if message.is_empty() {
                    return;
                }
                self.server
                    .chat_send_from_to(message, &from.login, vec![])
                    .await
                    .expect("failed to forward chat message");
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
                        display_name: &diff.info.display_name.formatted,
                    }),
                    RemovePlayer | RemoveSpectator | RemovePureSpectator => Some(Leaving {
                        display_name: &diff.info.display_name.formatted,
                    }),
                    _ => None,
                }
            }

            BeginOutro => {
                let vote_duration = self.config.vote_duration().await;
                let min_restart_vote_ratio = self.queue.lock().await.min_restart_vote_ratio;
                Some(VoteNow {
                    duration: vote_duration,
                    threshold: min_restart_vote_ratio,
                })
            }

            FinishRun(PbDiff {
                new_pos,
                pos_gained,
                new_record: Some(new_record),
                ..
            }) if *pos_gained > 0 && *new_pos <= MAX_ANNOUNCED_RECORD => Some(TopRecord {
                player_display_name: &new_record.player_display_name.formatted,
                new_map_rank: *new_pos,
                millis: new_record.millis as usize,
            }),

            FinishRun(PbDiff {
                new_pos,
                pos_gained,
                new_record: Some(new_record),
                millis_diff: Some(diff),
                ..
            }) if *pos_gained == 0 && *diff < 0 && *new_pos <= MAX_ANNOUNCED_RECORD_IMPROVEMENT => {
                Some(TopRecordImproved {
                    player_display_name: &new_record.player_display_name.formatted,
                    map_rank: *new_pos,
                    millis: new_record.millis as usize,
                })
            }

            NewPlaylist(PlaylistDiff::AppendNew(map)) => Some(NewMap {
                name: &map.name.formatted,
                author: &map.author_display_name.formatted,
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
                                display_name: &diff.player_display_name.formatted,
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
