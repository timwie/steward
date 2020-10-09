use std::collections::HashMap;

use serde::Deserialize;

use crate::api::structs::*;
use crate::api::Callback;
use crate::xml::{Call, Value};
use crate::{ModeScriptSectionCallback, PlayloopCallback, SCRIPT_API_VERSION};

/// Matches calls by their method name to their respective `Callback` variant.
///
/// Selectively ignores callbacks that we don't use.
///
/// Logs a warning on ignored callbacks that were not explicitly ignored.
///
/// # Panics
/// Panics if we recognized the name of a callback, but expected different parameters.
pub(in crate) fn to_callback(call: &Call) -> ReceivedCallback {
    log::debug!("callback: {:#?}", &call);

    let maybe_cb = if &call.name == "ManiaPlanet.ModeScriptCallbackArray" {
        forward_script_callback(call)
    } else {
        to_regular_callback(call)
    };

    let mut callback = match maybe_cb {
        Some(cb) => cb,
        None => return ReceivedCallback::Ignored,
    };

    let maybe_response_id = match &mut callback {
        Callback::Scores(scores) => scores.response_id.take(),
        Callback::PauseStatus(status) => status.response_id.take(),
        Callback::WarmupStatus(status) => status.response_id.take(),
        _ => None,
    };

    match maybe_response_id {
        None => ReceivedCallback::Unprompted(callback),
        Some(response_id) => ReceivedCallback::Prompted {
            callback,
            response_id,
        },
    }
}

pub enum ReceivedCallback {
    /// Received a callback that is not represented by the `Callback` enum.
    Ignored,

    /// Received a callback that was sent by the game server or mode script.
    Unprompted(Callback),

    /// Received a callback that was triggered by a mode script method call.
    Prompted {
        callback: Callback,
        response_id: String,
    },
}

fn to_regular_callback(call: &Call) -> Option<Callback> {
    use Callback::*;
    use PlayloopCallback::*;
    use Value::*;

    match call.name.as_ref() {
        "ManiaPlanet.PlayerChat" => {
            if let [Int(uid), String(login), String(msg), Bool(_is_registered_cmd)] = &call.args[..]
            {
                return Some(PlayerChat {
                    from_uid: *uid,
                    from_login: login.clone(),
                    message: msg.clone(),
                });
            }
        }

        "ManiaPlanet.PlayerDisconnect" => {
            if let [String(login), String(_reason)] = &call.args[..] {
                return Some(PlayerDisconnect {
                    login: login.clone(),
                });
            }
        }

        "ManiaPlanet.PlayerInfoChanged" => {
            if let [Struct(info)] = &call.args[..] {
                let info = from_arg(&call, Struct(info.clone()));
                return Some(PlayerInfoChanged(info));
            }
        }

        "ManiaPlanet.PlayerManialinkPageAnswer" => {
            if let [Int(uid), String(login), String(answer), Array(entries)] = &call.args[..] {
                let entries: HashMap<std::string::String, std::string::String> = entries
                    .iter()
                    .map(|val| {
                        let entry: ManialinkEntry = from_arg(&call, val.clone());
                        (entry.name, entry.value)
                    })
                    .collect();

                let answer = PlayerManialinkEvent {
                    answer: answer.clone(),
                    entries,
                };

                return Some(PlayerAnswered {
                    from_uid: *uid,
                    from_login: login.clone(),
                    answer,
                });
            }
        }

        "ManiaPlanet.MapListModified" => {
            if let [Int(curr_idx), Int(next_idx), Bool(playlist_modified)] = &call.args[..] {
                return Some(PlaylistChanged {
                    curr_idx: Some(*curr_idx).filter(|i| *i >= 0),
                    next_idx: *next_idx,
                    playlist_modified: *playlist_modified,
                });
            }
        }

        "TrackMania.PlayerIncoherence" => {
            if let [Int(_uid), String(login)] = &call.args[..] {
                return Some(Playloop(Incoherence {
                    login: login.clone(),
                }));
            }
        }

        "ManiaPlanet.BeginMap"
        | "ManiaPlanet.BeginMatch"
        | "ManiaPlanet.EndMatch"
        | "ManiaPlanet.EndMap"
        | "ManiaPlanet.PlayerConnect"
        | "ManiaPlanet.StatusChanged"
        | "TrackMania.PlayerCheckpoint"
        | "TrackMania.PlayerFinish" => {
            // ignore without logging
            return None;
        }
        _ => {
            log::warn!("ignored callback {:#?}", call);
            return None;
        }
    }

    panic!("unexpected signature for {:#?}", call)
}

fn from_arg<T>(call: &Call, value: Value) -> T
where
    T: serde::de::DeserializeOwned,
{
    crate::xml::from_value(value)
        .unwrap_or_else(|err| panic!("unexpected signature for {:#?}: {}", call, err))
}

fn forward_script_callback(call: &Call) -> Option<Callback> {
    use Callback::*;
    use ModeScriptSectionCallback::*;
    use PlayloopCallback::*;

    match script_callback_name(call) {
        "Maniaplanet.StartServer_Start" => {
            let data: StartServerEvent = from_script_callback(call);
            Some(ModeScriptSection(PreStartServer {
                restarted_script: data.restarted,
                changed_script: data.mode.updated,
            }))
        }

        "Maniaplanet.StartServer_End" => Some(ModeScriptSection(PostStartServer)),

        "Maniaplanet.StartMatch_Start" => Some(ModeScriptSection(PreStartMatch)),

        "Maniaplanet.StartMatch_End" => Some(ModeScriptSection(PostStartMatch)),

        "Maniaplanet.LoadingMap_Start" => {
            let data: LoadingMapEvent = from_script_callback(call);
            Some(ModeScriptSection(PreLoadMap {
                is_restart: data.restarted,
            }))
        }

        "Maniaplanet.LoadingMap_End" => Some(ModeScriptSection(PostLoadMap)),

        "Maniaplanet.StartMap_Start" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PreStartMap {
                nth_map_in_match: data.count,
            }))
        }

        "Trackmania.WarmUp.StartRound" => {
            let status: WarmupRoundStatus = from_script_callback(call);
            Some(ModeScriptSection(StartWarmupRound(status)))
        }

        "Trackmania.WarmUp.EndRound" => {
            let status: WarmupRoundStatus = from_script_callback(call);
            Some(ModeScriptSection(EndWarmupRound(status)))
        }

        "Maniaplanet.StartMap_End" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PostStartMap {
                nth_map_in_match: data.count,
            }))
        }

        "Maniaplanet.StartRound_Start" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PreStartRound {
                nth_round_on_map: data.count,
            }))
        }
        "Maniaplanet.StartRound_End" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PostStartRound {
                nth_round_on_map: data.count,
            }))
        }

        "Maniaplanet.StartPlayLoop" => Some(ModeScriptSection(StartPlayloop)),

        "Maniaplanet.EndPlayLoop" => Some(ModeScriptSection(EndPlayloop)),

        "Maniaplanet.EndRound_Start" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PreEndRound {
                nth_round_on_map: data.count,
            }))
        }

        "Maniaplanet.EndRound_End" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PostEndRound {
                nth_round_on_map: data.count,
            }))
        }

        "Maniaplanet.EndMap_Start" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PreEndMap {
                nth_map_in_match: data.count,
            }))
        }

        "Maniaplanet.EndMap_End" => {
            let data: CountedSectionEvent = from_script_callback(call);
            Some(ModeScriptSection(PostEndMap {
                nth_map_in_match: data.count,
            }))
        }

        "Maniaplanet.UnloadingMap_Start" => Some(ModeScriptSection(PreUnloadMap)),

        "Maniaplanet.UnloadingMap_End" => Some(ModeScriptSection(PostUnloadMap)),

        "Maniaplanet.EndMatch_Start" => Some(ModeScriptSection(PreEndMatch)),

        "Maniaplanet.EndMatch_End" => Some(ModeScriptSection(PostEndMatch)),

        "Maniaplanet.EndServer_Start" => Some(ModeScriptSection(PreEndServer)),

        "Maniaplanet.EndServer_End" => Some(ModeScriptSection(PostEndServer)),

        "Trackmania.Scores" => {
            let scores: crate::api::structs::Scores = from_script_callback(call);
            match scores.section {
                None => Some(Scores(scores)),
                Some(ScoresSection::PreEndRound) => {
                    Some(ModeScriptSection(PreEndRoundScores(scores)))
                }
                Some(ScoresSection::EndRound) => Some(ModeScriptSection(EndRoundScores(scores))),
                Some(ScoresSection::EndMap) => Some(ModeScriptSection(EndMapScores(scores))),
                Some(ScoresSection::EndMatch) => Some(ModeScriptSection(EndMatchScores(scores))),
            }
        }

        "Trackmania.Champion.Scores" => {
            let scores: ChampionEndRoundEvent = from_script_callback(call);
            Some(ModeScriptSection(EndRoundChampionScores(scores)))
        }

        "Trackmania.Knockout.Elimination" => {
            let elims: KnockoutEndRoundEvent = from_script_callback(call);
            Some(ModeScriptSection(EndRoundKnockoutEliminations(elims)))
        }

        "Maniaplanet.Pause.Status" => {
            let status: crate::structs::PauseStatus = from_script_callback(call);
            Some(PauseStatus(status))
        }

        "Trackmania.Event.StartLine" => {
            // Since TMNext, "Trackmania.Event.StartLine" is not triggered consistently,
            // but only when prior to spawning, the run outro was not skipped (this includes
            // the very first spawn for instance)
            let ev: GenericScriptEvent = from_script_callback(call);
            Some(Playloop(StartLine { login: ev.login }))
        }

        "Trackmania.Event.SkipOutro" => {
            // Since TMNext, "Trackmania.Event.StartCountdown" is never triggered,
            // but we know that the countdown will appear for players directly following
            // this event. "Trackmania.Event.StartLine" will *not* be triggered after
            // this event.
            let ev: GenericScriptEvent = from_script_callback(call);
            Some(Playloop(SkipOutro { login: ev.login }))
        }

        "Trackmania.Event.Respawn" => {
            let ev: CheckpointRespawnEvent = from_script_callback(call);
            Some(Playloop(CheckpointRespawn(ev)))
        }

        "Trackmania.Event.GiveUp" => {
            let ev: GenericScriptEvent = from_script_callback(call);
            Some(Playloop(GiveUp { login: ev.login }))
        }

        "Trackmania.Event.WayPoint" => {
            let event: CheckpointEvent = from_script_callback(call);
            let cb = if event.race_time_millis > 0 {
                Playloop(Checkpoint(event))
            } else {
                Playloop(Incoherence {
                    login: event.player_login,
                })
            };
            Some(cb)
        }

        "Trackmania.WarmUp.Status" => {
            let status: crate::structs::WarmupStatus = from_script_callback(call);
            Some(WarmupStatus(status))
        }

        "XmlRpc.AllApiVersions" => {
            #[derive(Deserialize)]
            struct AllApiVersions {
                pub latest: String,
            }
            let versions: AllApiVersions = from_script_callback(call);

            if versions.latest != SCRIPT_API_VERSION {
                log::warn!("not using latest script API version {}", &versions.latest);
            }

            None
        }

        "Maniaplanet.StartTurn_Start"
        | "Maniaplanet.StartTurn_End"
        | "Maniaplanet.EndTurn_Start"
        | "Maniaplanet.EndTurn_End"
        | "Maniaplanet.Podium_Start"
        | "Maniaplanet.Podium_End" => {
            // ignore without logging
            None
        }

        _ => {
            log::warn!("ignored script callback {:#?}", call);
            None
        }
    }
}

/// Deserialize data from the first JSON parameter of a script callback.
fn from_script_callback<'a, T>(call: &'a Call) -> T
where
    T: Deserialize<'a>,
{
    if let [Value::String(_cb_name), Value::Array(cb_args)] = &call.args[..] {
        let args: Vec<&str> = cb_args
            .iter()
            .map(|v| match v {
                Value::String(str) => str.as_ref(),
                _ => panic!("unexpected signature for {:#?}", call),
            })
            .collect();

        let first_arg = match args.into_iter().next() {
            Some(str) => str,
            None => panic!("unexpected signature for {:#?}", call),
        };

        return serde_json::from_str(first_arg)
            .unwrap_or_else(|err| panic!("unexpected signature for {:#?}: {}", call, err));
    }
    panic!("unexpected signature for {:#?}", call)
}

fn script_callback_name(call: &Call) -> &str {
    match call.args.first() {
        Some(Value::String(name)) => name,
        _ => panic!("unexpected signature for {:#?}", call),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ManialinkEntry {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct GenericScriptEvent {
    pub login: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct StartServerEvent {
    pub restarted: bool,
    pub mode: StartServerEventMode,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct StartServerEventMode {
    pub updated: bool,
    pub name: String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct LoadingMapEvent {
    pub restarted: bool,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct CountedSectionEvent {
    pub count: i32,
}
