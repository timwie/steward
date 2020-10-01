use std::collections::HashMap;

use serde::Deserialize;

use crate::api::structs::*;
use crate::api::Callback;
use crate::xml::{from_value, Call, Value};

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
        Callback::Scores { scores } => scores.response_id.take(),
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
    use Value::*;

    // Deserialize a value to T, and panic if it fails.
    // Using a macro since there are no generic closures.
    macro_rules! de {
        ($val:expr) => {
            from_value($val)
                .unwrap_or_else(|err| panic!("unexpected args for {}: {}", call.name, err))
        };
    }

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
                let info = de!(Struct(info.clone()));
                return Some(PlayerInfoChanged(info));
            }
        }

        "ManiaPlanet.PlayerManialinkPageAnswer" => {
            if let [Int(uid), String(login), String(answer), Array(entries)] = &call.args[..] {
                let entries: HashMap<std::string::String, std::string::String> = entries
                    .iter()
                    .map(|val| {
                        let entry = from_value::<ManialinkEntry>(val.clone())
                            .unwrap_or_else(|_| panic!("unexpected signature for {:?}", call));
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
                return Some(PlayerIncoherence {
                    login: login.clone(),
                });
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
            log::warn!("ignored callback {:?}", call);
            return None;
        }
    }

    panic!("unexpected signature for {:?}", call)
}

fn forward_script_callback(call: &Call) -> Option<Callback> {
    use crate::structs::ModeScriptSection::*;
    use Callback::*;
    use Value::*;

    // Deserialize JSON to T, and panic if it fails.
    // Using a macro since there are no generic closures.
    macro_rules! de {
        ($json_str:expr) => {
            serde_json::from_str($json_str)
                .unwrap_or_else(|err| panic!("unexpected args for {}: {}", call.name, err))
        };
    }

    if let [String(cb_name), Array(value_args)] = &call.args[..] {
        // All arguments of script callbacks are strings.
        let str_args: Vec<std::string::String> = value_args
            .iter()
            .map(|v| match v {
                String(str) => str.clone(),
                _ => panic!("expected only String args for {}", call.name),
            })
            .collect();

        return match cb_name.as_ref() {
            "Maniaplanet.StartServer_Start" => {
                let data: StartServerEvent = de!(&str_args[0]);
                Some(ModeScriptSection(PreStartServer {
                    restarted_script: data.restarted,
                    changed_script: data.mode.updated,
                }))
            }
            "Maniaplanet.StartServer_End" => Some(ModeScriptSection(PostStartServer)),
            "Maniaplanet.StartMatch_Start" => Some(ModeScriptSection(PreStartMatch)),
            "Maniaplanet.StartMatch_End" => Some(ModeScriptSection(PostStartMatch)),
            "Maniaplanet.LoadingMap_Start" => {
                let data: LoadingMapEvent = de!(&str_args[0]);
                Some(ModeScriptSection(PreLoadMap {
                    is_restart: data.restarted,
                }))
            }
            "Maniaplanet.LoadingMap_End" => Some(ModeScriptSection(PostLoadMap)),
            "Maniaplanet.StartMap_Start" => Some(ModeScriptSection(PreStartMap)),
            "Maniaplanet.StartMap_End" => Some(ModeScriptSection(PostStartMap)),
            "Maniaplanet.StartRound_Start" => Some(ModeScriptSection(PreStartRound)),
            "Maniaplanet.StartRound_End" => Some(ModeScriptSection(PostStartRound)),
            "Maniaplanet.StartPlayLoop" => Some(ModeScriptSection(PrePlayloop)),
            "Maniaplanet.EndPlayLoop" => Some(ModeScriptSection(PostPlayloop)),
            "Maniaplanet.EndRound_Start" => Some(ModeScriptSection(PreEndRound)),
            "Maniaplanet.EndRound_End" => Some(ModeScriptSection(PostEndRound)),
            "Maniaplanet.EndMap_Start" => Some(ModeScriptSection(PreEndMap)),
            "Maniaplanet.EndMap_End" => Some(ModeScriptSection(PostEndMap)),
            "Maniaplanet.UnloadingMap_Start" => Some(ModeScriptSection(PreUnloadMap)),
            "Maniaplanet.UnloadingMap_End" => Some(ModeScriptSection(PostUnloadMap)),
            "Maniaplanet.EndMatch_Start" => Some(ModeScriptSection(PreEndMatch)),
            "Maniaplanet.EndMatch_End" => Some(ModeScriptSection(PostEndMatch)),
            "Maniaplanet.EndServer_Start" => Some(ModeScriptSection(PreEndServer)),
            "Maniaplanet.EndServer_End" => Some(ModeScriptSection(PostEndServer)),

            "Maniaplanet.Pause.Status" => {
                let status: crate::structs::PauseStatus = de!(&str_args[0]);
                Some(PauseStatus(status))
            }

            "Trackmania.Champion.Scores" => {
                let scores: ChampionEndRoundEvent = de!(&str_args[0]);
                Some(ChampionRoundEnd(scores))
            }

            "Trackmania.Event.GiveUp" | "Trackmania.Event.SkipOutro" => {
                // Since TMNext, "Trackmania.Event.StartCountdown" is never triggered,
                // but we know that the countdown will appear for players directly following
                // these two events. "Trackmania.Event.StartLine" will *not* be triggered after
                // either of these events.
                let ev: GenericScriptEvent = de!(&str_args[0]);
                Some(PlayerCountdown { login: ev.login })
            }

            "Trackmania.Event.Respawn" => {
                let ev: CheckpointRespawnEvent = de!(&str_args[0]);
                return Some(PlayerCheckpointRespawn(ev));
            }

            "Trackmania.Event.StartLine" => {
                // Since TMNext, "Trackmania.Event.StartLine" is not triggered consistently,
                // but only when prior to spawning, the run outro was not skipped (this includes
                // the very first spawn for instance)
                let ev: GenericScriptEvent = de!(&str_args[0]);
                Some(PlayerStartline { login: ev.login })
            }

            "Trackmania.Event.WayPoint" => {
                let event: CheckpointEvent = de!(&str_args[0]);
                let cb = if event.race_time_millis > 0 {
                    PlayerCheckpoint(event)
                } else {
                    PlayerIncoherence {
                        login: event.player_login,
                    }
                };
                Some(cb)
            }

            "Trackmania.Knockout.Elimination" => {
                let elims: KnockoutEndRoundEvent = de!(&str_args[0]);
                Some(KnockoutRoundEnd(elims))
            }

            "Trackmania.Scores" => {
                let scores: crate::api::structs::Scores = de!(&str_args[0]);
                Some(Scores { scores })
            }

            "Trackmania.WarmUp.EndRound" => {
                let status: WarmupRoundStatus = de!(&str_args[0]);
                Some(WarmupEnd(status))
            }

            "Trackmania.WarmUp.StartRound" => {
                let status: WarmupRoundStatus = de!(&str_args[0]);
                Some(WarmupBegin(status))
            }

            "Trackmania.WarmUp.Status" => {
                let status: crate::structs::WarmupStatus = de!(&str_args[0]);
                Some(WarmupStatus(status))
            }

            "Maniaplanet.StartTurn_Start"
            | "Maniaplanet.StartTurn_End"
            | "Maniaplanet.EndTurn_Start"
            | "Maniaplanet.EndTurn_End"
            | "Maniaplanet.Podium_Start"
            | "Maniaplanet.Podium_End"
            | "Trackmania.Event.OnPlayerAdded"
            | "Trackmania.Event.OnPlayerRemoved"
            | "Trackmania.Event.Stunt" => {
                // ignore without logging
                None
            }
            _ => {
                log::warn!("ignored script callback {:?}", call);
                None
            }
        };
    }

    panic!("unexpected signature for {:?}", call)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ManialinkEntry {
    pub name: std::string::String,
    pub value: std::string::String,
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
struct GenericScriptEvent {
    pub login: std::string::String,
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
