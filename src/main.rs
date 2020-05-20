#[macro_use]
extern crate include_dir;

mod action;
mod command;
mod config;
mod controller;
mod database;
mod event;
mod ingame;
mod message;
mod network;
mod widget;

/// The controller's entry-point.
///
/// If no game server is running, this function will periodically try
/// to connect. Whenever the game server stops, this function will panic.
#[tokio::main]
async fn main() {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::time::delay_for;

    use config::Config;
    use controller::Controller;
    use database::db_connect;
    use ingame::{RpcConnection, Server};

    env_logger::init(); // Use log::* to write to stdout/err

    let config = Config::read_from_env().await;

    let db = db_connect(&config).await;

    const RETRY_CONNECT_AFTER_SECS: u64 = 1;

    log::info!("waiting for connection...");
    let mut conn = loop {
        match RpcConnection::new(&config.rpc_address).await {
            None => {
                delay_for(Duration::from_secs(RETRY_CONNECT_AFTER_SECS)).await;
                log::debug!("waiting for connection...");
            }
            Some(conn) => break conn,
        }
    };
    log::info!("got connection");

    let server = Arc::new(conn.client.clone()) as Arc<dyn Server>;

    let controller = Controller::init(config, server, db).await;

    log::info!("running callback loop...");
    loop {
        let next_callback = conn
            .callbacks
            .recv()
            .await
            .expect("callback receiver disconnected");
        controller.on_server_event(next_callback).await;
    }

    // Here we don't care about explicitly joining the TCP ('conn.tcp_handle')
    // or msg loop ('conn.msg_handle'), and simply run the callback loop in the
    // main task until something breaks.
    // We could properly unwind if we used 'panic = "unwind"', and joined
    // all tasks/threads accordingly.
}
