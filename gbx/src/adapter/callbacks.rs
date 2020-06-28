use std::collections::HashMap;

use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender as Sender;

use crate::api::structs::*;
use crate::api::Callback;
use crate::xml::{from_value, Call, Value};

/// Matches calls by their method name to their respective `Callback` variant,
/// which is then sent with the given `Sender`.
///
/// Selectively ignores callbacks that we don't use.
///
/// Logs a warning on ignored callbacks that were not explicitly ignored.
///
/// # Panics
/// Panics if we recognized the name of a callback, but expected different parameters.
///
/// Panics if the callback `Receiver` was dropped.
pub fn forward_callback(cb_out: &Sender<Callback>, call: Call) -> CallbackType {
    log::debug!("callback: {:#?}", &call);
    if &call.name == "ManiaPlanet.ModeScriptCallbackArray" {
        forward_script_callback(cb_out, call)
    } else {
        forward_regular_callback(cb_out, call)
    }
}

/// A callback is either triggered by the game/script itself,
/// or triggered explicitly, in which case it includes a `response_id`.
pub enum CallbackType {
    Prompted { response_id: String },
    Unprompted,
}

fn forward_regular_callback(cb_out: &Sender<Callback>, call: Call) -> CallbackType {
    use Callback::*;
    use Value::*;

    let success = |cb: Callback| -> CallbackType {
        cb_out.send(cb).expect("callback receiver was dropped");
        CallbackType::Unprompted
    };

    // Deserialize a value to T, and panic if it fails.
    // Using a macro since there are no generic closures.
    macro_rules! de {
        ($val:expr) => {
            from_value($val)
                .unwrap_or_else(|err| panic!("unexpected args for {}: {}", call.name, err))
        };
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Entry {
        pub name: std::string::String,
        pub value: std::string::String,
    }

    match call.name.as_ref() {
        "ManiaPlanet.EndMatch" => {
            if let [Array(_rankings), Int(_winner_team)] = &call.args[..] {
                return success(RaceEnd);
            }
        }
        "ManiaPlanet.MapListModified" => {
            if let [Int(curr_idx), Int(next_idx), Bool(_is_list_modified)] = &call.args[..] {
                return success(PlaylistChanged {
                    curr_idx: Some(*curr_idx).filter(|i| *i >= 0),
                    next_idx: *next_idx,
                });
            }
        }
        "ManiaPlanet.PlayerChat" => {
            if let [Int(uid), String(login), String(msg), Bool(_is_registered_cmd)] = &call.args[..]
            {
                return success(PlayerChat {
                    from_uid: *uid,
                    from_login: login.clone(),
                    message: msg.clone(),
                });
            }
        }
        "ManiaPlanet.PlayerDisconnect" => {
            if let [String(login), String(_reason)] = &call.args[..] {
                return success(PlayerDisconnect {
                    login: login.clone(),
                });
            }
        }
        "ManiaPlanet.PlayerInfoChanged" => {
            if let [Struct(info)] = &call.args[..] {
                return success(PlayerInfoChanged {
                    info: de!(Struct(info.clone())),
                });
            }
        }
        "ManiaPlanet.PlayerManialinkPageAnswer" => {
            if let [Int(uid), String(login), String(answer), Array(entries)] = &call.args[..] {
                let entries: HashMap<std::string::String, std::string::String> = entries
                    .iter()
                    .map(|val| {
                        let entry = from_value::<Entry>(val.clone())
                            .unwrap_or_else(|_| panic!("unexpected signature for {:?}", call));
                        (entry.name, entry.value)
                    })
                    .collect();

                let answer = PlayerAnswer {
                    answer: answer.clone(),
                    entries,
                };

                return success(PlayerAnswered {
                    from_uid: *uid,
                    from_login: login.clone(),
                    answer,
                });
            }
        }
        "ManiaPlanet.BeginMap"
        | "ManiaPlanet.BeginMatch"
        | "ManiaPlanet.EndMap"
        | "ManiaPlanet.StatusChanged"
        | "TrackMania.PlayerCheckpoint"
        | "TrackMania.PlayerFinish"
        | "TrackMania.PlayerIncoherence"
        | "ManiaPlanet.PlayerConnect" => {
            // ignore without logging
            return CallbackType::Unprompted;
        }
        _ => {
            log::warn!("ignored callback {:?}", call);
            return CallbackType::Unprompted;
        }
    }

    panic!("unexpected signature for {:?}", call)
}

fn forward_script_callback(cb_out: &Sender<Callback>, call: Call) -> CallbackType {
    use Callback::*;
    use Value::*;

    let send = |cb: Callback| {
        cb_out.send(cb).expect("callback receiver was dropped");
    };

    // Deserialize JSON to T, and panic if it fails.
    // Using a macro since there are no generic closures.
    macro_rules! de {
        ($json_str:expr) => {
            serde_json::from_str($json_str)
                .unwrap_or_else(|err| panic!("unexpected args for {}: {}", call.name, err))
        };
    }

    /// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#trackmaniaeventgiveup
    #[derive(Deserialize, Debug, PartialEq, Clone)]
    struct ScriptEventData {
        pub login: std::string::String,
    }

    /// Reference: https://github.com/maniaplanet/script-xmlrpc/blob/master/XmlRpcListing.md#maniaplanetloadingmap_start
    #[derive(Deserialize, Debug, PartialEq, Clone)]
    struct LoadingMapEvent {
        pub restarted: bool,
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

        let (cb, cb_type) = match cb_name.as_ref() {
            "Maniaplanet.LoadingMap_Start" => {
                let data: LoadingMapEvent = de!(&str_args[0]);
                let cb = MapLoad {
                    is_restart: data.restarted,
                };
                (cb, CallbackType::Unprompted)
            }
            "Maniaplanet.UnloadingMap_Start" => (MapUnload, CallbackType::Unprompted),
            "Trackmania.Event.StartLine" => {
                let data: ScriptEventData = de!(&str_args[0]);
                let cb = RunStartline {
                    player_login: data.login,
                };
                (cb, CallbackType::Unprompted)
            }
            "Trackmania.Event.WayPoint" => {
                let cb = RunCheckpoint {
                    event: de!(&str_args[0]),
                };
                (cb, CallbackType::Unprompted)
            }
            "Trackmania.Scores" => {
                let scores: Scores = de!(&str_args[0]);
                let cb_type = match scores.response_id.as_ref() {
                    "" => CallbackType::Unprompted,
                    response_id => CallbackType::Prompted {
                        response_id: response_id.to_string(),
                    },
                };
                let cb = MapScores { scores };
                (cb, cb_type)
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
            | "Trackmania.Event.GiveUp"
            | "Trackmania.Event.OnPlayerAdded"
            | "Trackmania.Event.OnPlayerRemoved"
            | "Trackmania.Event.Respawn"
            | "Trackmania.Event.StartCountdown"
            | "Trackmania.Event.Stunt"
            | "Maniaplanet.StartServer_Start" => {
                // ignore without logging
                return CallbackType::Unprompted;
            }
            _ => {
                log::warn!("ignored script callback {:?}", call);
                return CallbackType::Unprompted;
            }
        };

        send(cb);
        return cb_type;
    }

    panic!("unexpected signature for {:?}", call)
}
