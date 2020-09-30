mod chat;
mod compat;
mod config;
mod constants;
mod controller;
mod database;
mod event;
mod network;
mod server;
mod widget;

/// The controller's entry-point.
///
/// If no game server is running, this function will periodically try
/// to connect. Whenever the game server stops, this function will panic.
#[tokio::main]
async fn main() {
    use std::sync::Arc;
    use std::time::Duration;

    use dotenv::dotenv;
    use tokio::time::delay_for;

    use config::Config;
    use controller::Controller;
    use database::db_connect;
    use server::{RpcConnection, Server};

    // Read environment variables from an '.env' file in the working directory.
    // We use these env vars:
    //  - RUST_LOG
    //  - STEWARD_CONFIG
    let using_env_file = dotenv().is_ok();

    env_logger::init(); // Use log::* to write to stderr

    if using_env_file {
        log::info!("using .env file")
    }

    let config = Config::load();

    let retry_after = Duration::from_secs(1);

    log::info!("waiting for dedicated server connection...");
    let mut conn = loop {
        match RpcConnection::new(&config.rpc_address).await {
            None => {
                delay_for(retry_after).await;
                log::debug!("waiting for dedicated server connection...");
            }
            Some(conn) => break conn,
        }
    };
    log::info!("got dedicated server connection");

    let server = Arc::new(conn.client.clone()) as Arc<dyn Server>;

    log::info!("waiting for database connection...");
    let db = loop {
        match db_connect(&config.postgres_connection, retry_after).await {
            None => log::debug!("waiting for database connection..."),
            Some(db) => break db,
        }
    };
    log::info!("got database connection");

    compat::prepare(&server, &db, &config).await;

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
