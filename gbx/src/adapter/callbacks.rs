use std::collections::HashMap;

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
pub fn to_callback(call: Call) -> ReceivedCallback {
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

fn to_regular_callback(call: Call) -> Option<Callback> {
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
        "ManiaPlanet.EndMatch" => {
            if let [Array(_rankings), Int(_winner_team)] = &call.args[..] {
                return Some(RaceEnd);
            }
        }
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
                return Some(PlayerInfoChanged {
                    info: de!(Struct(info.clone())),
                });
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

                let answer = PlayerAnswer {
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
        "TrackMania.PlayerIncoherence" => {
            if let [Int(_uid), String(login)] = &call.args[..] {
                return Some(RunIncoherence {
                    player_login: login.clone(),
                });
            }
        }
        "ManiaPlanet.BeginMap"
        | "ManiaPlanet.BeginMatch"
        | "ManiaPlanet.EndMap"
        | "ManiaPlanet.MapListModified"
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

fn forward_script_callback(call: Call) -> Option<Callback> {
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
            "Maniaplanet.LoadingMap_Start" => {
                let data: LoadingMapEvent = de!(&str_args[0]);
                Some(MapLoad {
                    is_restart: data.restarted,
                })
            }

            "Maniaplanet.Pause.Status" => {
                let status: WarmupOrPauseStatus = de!(&str_args[0]);
                Some(PauseStatus(status))
            }

            "Maniaplanet.UnloadingMap_Start" => Some(MapUnload),

            "ManiaPlanet.WarmUp.Status" | "Trackmania.WarmUp.Status" => {
                let status: WarmupOrPauseStatus = de!(&str_args[0]);
                Some(WarmupStatus(status))
            }

            "Trackmania.Event.GiveUp" | "Trackmania.Event.SkipOutro" => {
                // Since TMNext, "Trackmania.Event.StartCountdown" is never triggered,
                // but we know that the countdown will appear for players directly following
                // these two events. "Trackmania.Event.StartLine" will *not* be triggered after
                // either of these events.
                let ev: GenericScriptEvent = de!(&str_args[0]);
                Some(RunCountdown {
                    player_login: ev.login,
                })
            }

            "Trackmania.Event.StartLine" => {
                // Since TMNext, "Trackmania.Event.StartLine" is not triggered consistently,
                // but only when prior to spawning, the run outro was not skipped (this includes
                // the very first spawn for instance)
                let ev: GenericScriptEvent = de!(&str_args[0]);
                Some(RunStartline {
                    player_login: ev.login,
                })
            }

            "Trackmania.Event.WayPoint" => {
                let event: CheckpointEvent = de!(&str_args[0]);
                let cb = if event.race_time_millis > 0 {
                    RunCheckpoint { event }
                } else {
                    RunIncoherence {
                        player_login: event.player_login,
                    }
                };
                Some(cb)
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

            "Maniaplanet.ChannelProgression_End"
            | "Maniaplanet.ChannelProgression_Start"
            | "Maniaplanet.EndMap_End"
            | "Maniaplanet.EndMap_Start"
            | "Maniaplanet.EndMatch_End"
            | "Maniaplanet.EndMatch_Start"
            | "Maniaplanet.EndPlayLoop"
            | "Maniaplanet.EndRound_End"
            | "Maniaplanet.EndRound_Start"
            | "Maniaplanet.EndTurn_End"
            | "Maniaplanet.EndTurn_Start"
            | "Maniaplanet.LoadingMap_End"
            | "Maniaplanet.Podium_End"
            | "Maniaplanet.Podium_Start"
            | "Maniaplanet.StartMap_End"
            | "Maniaplanet.StartMap_Start"
            | "Maniaplanet.StartMatch_End"
            | "Maniaplanet.StartMatch_Start"
            | "Maniaplanet.StartPlayLoop"
            | "Maniaplanet.StartRound_End"
            | "Maniaplanet.StartRound_Start"
            | "Maniaplanet.StartServer_End"
            | "Maniaplanet.StartTurn_End"
            | "Maniaplanet.StartTurn_Start"
            | "Maniaplanet.UnloadingMap_End"
            | "Trackmania.Event.OnPlayerAdded"
            | "Trackmania.Event.OnPlayerRemoved"
            | "Trackmania.Event.Respawn"
            | "Trackmania.Event.Stunt"
            | "Maniaplanet.StartServer_Start" => {
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
