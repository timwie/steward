use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::thread::JoinHandle as ThreadHandle;
use std::time::Duration;

use anyhow::{Context, Result};
use byteorder::{ByteOrder, LittleEndian};
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver as Receiver, UnboundedSender as Sender,
};
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio::task::JoinHandle as TaskHandle;

use crate::adapter::callbacks::{forward_callback, CallbackType};
use crate::xml::*;
use crate::Callback;

/// The variants of this enum are used to control the thread
/// that is matching requests with responses, and executes
/// callbacks.
#[derive(Debug)]
enum Msg {
    /// This message signals that an XML-RPC call has been made,
    /// and that once the response is received, it needs to be sent
    /// back to the calling thread.
    AwaitResponse(AwaitResponseData),

    /// This message signals that a previous XML-RPC call is supposed
    /// to trigger a callback, and to notify the calling thread once it
    /// was received.
    AwaitCallback(AwaitCallbackData),

    /// This message signals that we have received a callback.
    ///
    /// In this instance, the controller acts as an XML-RPC server,
    /// that receives a method call, but does not send a method response
    /// back. This is how we get notified of events on the game server.
    FulfillCallback { call: Call },

    /// This message signals that we have received an XML-RPC method
    /// response that needs to be sent to the calling thread.
    ///
    /// It can be matched to a made call with the "handle" that was sent
    /// alongside the XML payload of the call.
    FulfillResponse { handle: u32, response: Response },
}

/// Data related to an XML-RPC method call.
///
/// A received method response can be matched to a made call with its
/// `handle` which is sent alongside the XML payload of the call.
///
/// The payload of the method response is forwarded to the one-shot sender,
/// so that when making a call, we can suspend a future until its result
/// is available.
#[derive(Debug)]
struct AwaitResponseData {
    handle: u32,
    eventual_response: oneshot::Sender<Response>,
}

/// If script methods produce results, they do not return them directly.
/// Instead, they trigger callbacks, in which the call parameters make
/// up the results.
///
/// The method call triggering the callback and the callback itself can
/// be matched by the `response_id` that is parameter of the method call,
/// and is contained in the callback parameters in some form.
///
/// This struct contains the `response_id` that was used for the triggering method call,
/// and a one-shot sender that receives a signal that the call was received.
/// If the call is never received, it usually means that the triggered
/// callback does not exist.
#[derive(Debug)]
struct AwaitCallbackData {
    response_id: String,
    eventual_callback: oneshot::Sender<AwaitCallbackResult>,
}

/// Using a script method to trigger a callback *does* produce a result indirectly,
/// but since these callbacks are also triggered by the script itself, it seems
/// easier to not return this result to the triggering caller, but to simply
/// "return" it via the callback. This result type is therefore empty.
#[derive(Debug)]
struct AwaitCallbackResult;

/// Open a TCP connection to the game server.
///
/// Will return an IO error if a connection could not be
/// established, which typically means there is no running server.
///
/// # Panics
/// Panics if reading from the stream fails or when
/// encountering an unexpected server protocol.
fn tcp_connect(addr: &str) -> Result<TcpStream, std::io::Error> {
    const SERVER_PROTOCOL: &str = "GBXRemote 2";

    let mut stream = TcpStream::connect(&addr)?;

    let mut protocol_name_length_bytes = [0; 4];
    stream
        .read_exact(&mut protocol_name_length_bytes[..])
        .expect("no TCP connection");
    let protocol_name_length = LittleEndian::read_u32(&protocol_name_length_bytes);

    let mut protocol_name_bytes = vec![0; protocol_name_length as usize];
    stream
        .read_exact(&mut protocol_name_bytes[..])
        .expect("no TCP connection");
    let protocol_name =
        std::str::from_utf8(&protocol_name_bytes).expect("server protocol was not UTF-8");

    if protocol_name == SERVER_PROTOCOL {
        Ok(stream)
    } else {
        panic!(
            "server uses protocol '{}', expected '{}'",
            protocol_name, SERVER_PROTOCOL
        );
    }
}

/// Spawns a thread that reads from the TCP connection to the game server,
/// and sends out either `Msg::FulfillResponse` or `Msg::FulfillCallback`
/// messages.
///
/// # Panics
/// This thread terminates with a panic
/// - when the message receiver was dropped,
/// - when the TCP connection is interrupted
/// - when reading/parsing failed
fn tcp_loop(tcp_stream: TcpStream, msg_out: Sender<Msg>) -> ThreadHandle<()> {
    fn try_loop(mut tcp_stream: TcpStream, msg_out: Sender<Msg>) -> anyhow::Result<()> {
        let mut u32_bytes = [0; 4];

        loop {
            tcp_stream
                .read_exact(&mut u32_bytes[..])
                .context("no TCP connection")?;
            let message_length = LittleEndian::read_u32(&u32_bytes);

            tcp_stream
                .read_exact(&mut u32_bytes[..])
                .context("no TCP connection")?;
            let message_handle = LittleEndian::read_u32(&u32_bytes);

            if message_length == 0 {
                continue;
            }

            let mut message_bytes = vec![0; message_length as usize];
            tcp_stream
                .read_exact(&mut message_bytes[..])
                .context("no TCP connection")?;

            let message =
                std::str::from_utf8(&message_bytes).context("tcp response was not UTF-8")?;

            let is_callback = message_handle & RESPONSE_MASK == 0;
            let msg = if is_callback {
                Msg::FulfillCallback {
                    call: read_method_call(message)
                        .context(format!("failed to parse method call {}", message))?,
                }
            } else {
                Msg::FulfillResponse {
                    handle: message_handle,
                    response: read_method_response(message)
                        .context(format!("failed to parse method response {}", message))?,
                }
            };
            msg_out.send(msg).context("msg receiver dropped")?;
        }
    }

    std::thread::spawn(move || {
        try_loop(tcp_stream, msg_out).unwrap(); // let it crash
    })
}

/// If the bit-and of a handle and this value equal 0,
/// the received data is a callback. If it equals 1,
/// it is a method response.
///
/// Handles greater than 0x8000_0000 are responses.
/// Handles lower than 0x8000_0000 are callbacks.
const RESPONSE_MASK: u32 = 0x8000_0000;

/// Send an XML-RPC method call to the game server.
///
/// # Panics
/// Panics if the TCP connection was closed or the call
/// could not be translated into XML.
async fn tcp_send(tcp_stream: &Arc<Mutex<TcpStream>>, call: &Call, call_handle: u32) {
    let call_bytes = write_method_call(call);

    let mut handle_bytes = [0; 4];
    LittleEndian::write_u32(&mut handle_bytes, call_handle);

    let mut length_bytes = [0; 4];
    LittleEndian::write_u32(&mut length_bytes, call_bytes.len() as u32);

    let msg = [&length_bytes[..], &handle_bytes[..], &call_bytes[..]].concat();

    tcp_stream
        .lock()
        .await
        .write_all(&msg)
        .expect("no TCP connection");
}

/// An XML-RPC client to the game server.
#[derive(Clone)]
pub struct RpcClient {
    /// A handle on the TCP stream between this controller
    /// and the game server.
    tcp_stream: Arc<Mutex<TcpStream>>,

    /// A reference to a global call handle that is increased
    /// for each method call, so that responses can be traced
    /// back to them.
    prev_call_handle: Arc<Mutex<u32>>,

    /// The `Sender` that feeds the message loop.
    msg_out: Sender<Msg>,
}

impl RpcClient {
    fn new(tcp_stream: TcpStream, msg_out: Sender<Msg>) -> RpcClient {
        RpcClient {
            msg_out,
            tcp_stream: Arc::new(Mutex::new(tcp_stream)),
            prev_call_handle: Arc::new(Mutex::new(RESPONSE_MASK)),
        }
    }

    /// Make an XML-RPC call, and let the caller handle faults.
    ///
    /// # Panics
    /// - when getting a different return type than expected
    /// - when failing to parse the return value
    /// - when the TCP connection was closed
    /// - when failing to compose the XML-RPC call
    /// - when a channel between threads was dropped
    pub(super) async fn call<T>(&self, call: Call) -> Result<T, Fault>
    where
        T: serde::de::DeserializeOwned,
    {
        let call_trace = call.clone();
        let value = self.call_response(call).await?;
        match from_value(value) {
            Ok(t) => Ok(t),
            Err(err) => panic!("unexpected return value for {:?}: {}", call_trace, err),
        }
    }

    /// Make an XML-RPC call, and do not expect a fault.
    ///
    /// # Panics
    /// - when encountering a fault after all
    /// - see also: `call` doc
    pub(super) async fn call_unwrap<T>(&self, call: Call) -> T
    where
        T: serde::de::DeserializeOwned,
    {
        let call_clone = call.clone();
        let res: Result<T, Fault> = self.call(call).await;
        match res {
            Ok(t) => t,
            Err(fault) => panic!("unexpected fault {:?} for call {:?}", fault, call_clone),
        }
    }

    async fn call_response(&self, call: Call) -> Response {
        let handle = self.next_handle().await;

        let (resp_out, resp_in) = oneshot::channel::<Response>();

        let data = AwaitResponseData {
            handle,
            eventual_response: resp_out,
        };
        self.msg_out
            .send(Msg::AwaitResponse(data))
            .expect("msg receiver was dropped");

        log::debug!("call {}: {:?}", &handle, &call);

        tcp_send(&self.tcp_stream, &call, handle).await;

        let response = resp_in.await.expect("response sender was dropped");

        log::debug!("call {} response: {:?}", &handle, &response);

        response
    }

    /// Make an XML-RPC script call, and expect a callback in return.
    /// This function will suspend until the call was received, but *not*
    /// until after its execution.
    ///
    /// # Panics
    /// - refer to `call` doc
    /// - when the requested callback doesn't exist
    pub(super) async fn trigger_callback(&self, response_id: String, call: Call) {
        let (res_out, res_in) = oneshot::channel::<AwaitCallbackResult>();

        let data = AwaitCallbackData {
            response_id: response_id.clone(),
            eventual_callback: res_out,
        };
        self.msg_out
            .send(Msg::AwaitCallback(data))
            .expect("msg receiver was dropped");

        // We always get a Unit response, which doesn't tell us
        // whether the requested callback actually exists...
        let _response = self.call_response(call).await;

        // ... Instead, we will add a timeout when waiting for the callback,
        // and assume it doesn't exist once that timeout was exceeded.
        let await_callback = async { res_in.await.expect("callback result sender was dropped") };
        tokio::time::timeout(
            Duration::from_secs(if cfg!(debug_assertions) {
                DEBUG_CALLBACK_TIMEOUT_SECS
            } else {
                CALLBACK_TIMEOUT_SECS
            }),
            await_callback,
        )
        .await
        .expect("callback was never triggered");
    }

    async fn next_handle(&self) -> u32 {
        let mut prev = self.prev_call_handle.lock().await;
        *prev += 1;
        if *prev == 0xffff_ffff {
            // just in case we make two billion calls ;)
            *prev = 0x8000_0000;
        }
        *prev
    }
}

/// Timeout duration when waiting for triggered callbacks.
///
/// This value should be longer than you would expect the game server
/// to take to respond. In fact, we can just set this very high, since
/// it should be a very rare error.
const CALLBACK_TIMEOUT_SECS: u64 = 30;

/// A shorter replacement for `CALLBACK_TIMEOUT_SECS` for quicker debugging.
const DEBUG_CALLBACK_TIMEOUT_SECS: u64 = 1;

/// This task consumes all `Msg`s, and produces `Callback`s, as well as
/// responses to waiting receivers of an `RpcClient`.
///
/// # Panics
/// This task terminates with a panic once every message sender was dropped.
/// This means that this task does not necessarily terminate with the TCP loop -
/// it will only terminate once any `RpcClient`s that still hold a sender are dropped.
fn msg_loop(mut msg_in: Receiver<Msg>, cb_out: Sender<Callback>) -> TaskHandle<()> {
    tokio::spawn(async move {
        let mut waiting_calls: HashMap<u32, AwaitResponseData> = HashMap::new();
        let mut waiting_cbs: HashMap<String, AwaitCallbackData> = HashMap::new();

        loop {
            match msg_in.recv().await.expect("message receiver disconnected") {
                Msg::AwaitResponse(data) => {
                    waiting_calls.insert(data.handle, data);
                }
                Msg::AwaitCallback(data) => {
                    waiting_cbs.insert(data.response_id.clone(), data);
                }
                Msg::FulfillResponse { handle, response } => {
                    let _send_result = waiting_calls
                        .remove(&handle)
                        .expect("failed to match incoming XML-RPC response handle")
                        .eventual_response
                        .send(response);
                }
                Msg::FulfillCallback { call } => match forward_callback(&cb_out, call) {
                    CallbackType::Unprompted => {}
                    CallbackType::Prompted { response_id } => {
                        let data = waiting_cbs
                            .remove(&response_id)
                            .expect("failed to match callback response_id");
                        let _send_result = data.eventual_callback.send(AwaitCallbackResult {});
                    }
                },
            } // end match
        } // end loop
    }) // end spawn
}

/// A connection to the game server consists of
/// - a cloneable client to make calls with
/// - a receiver to consume callbacks with
/// - handles for threads that run the client & receiver
pub struct RpcConnection {
    pub client: RpcClient,
    pub callbacks: Receiver<Callback>,
    pub tcp_handle: ThreadHandle<()>,
    pub msg_handle: TaskHandle<()>,
}

impl RpcConnection {
    /// Try to connect to the game server.
    ///
    /// # Panics
    /// - when the given socket address is invalid
    /// - when failing to clone the TCP stream handle
    pub async fn new(addr: &str) -> Option<RpcConnection> {
        log::debug!("using XML-RPC address: {}", addr);

        let tcp_stream = match tcp_connect(&addr) {
            Ok(stream) => stream,
            Err(err) => {
                log::debug!("cannot connect: {}", err);
                return None;
            }
        };
        let (msg_out, msg_in) = unbounded_channel();
        let (cb_out, cb_in) = unbounded_channel();

        let tcp_write_stream = tcp_stream;
        let tcp_read_stream = tcp_write_stream
            .try_clone()
            .expect("failed to clone handle on TCP stream");

        let msg_from_server = msg_out;
        let msg_from_controller = msg_from_server.clone();

        Some(RpcConnection {
            client: RpcClient::new(tcp_write_stream, msg_from_controller),
            callbacks: cb_in,
            tcp_handle: tcp_loop(tcp_read_stream, msg_from_server),
            msg_handle: msg_loop(msg_in, cb_out),
        })
    }
}
