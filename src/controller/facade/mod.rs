use std::sync::Arc;

use crate::chat::ServerMessage;
use crate::config::Config;
use crate::controller::*;
use crate::database::DatabaseClient;
use crate::server::{Calls, Server};

mod on_action;
mod on_command;
mod on_config_change;
mod on_controller_event;
mod on_server_event;

/// This facade hides all specific controllers behind one interface
/// that can react to server events.
#[derive(Clone)]
pub struct Controller {
    server: Server,
    db: DatabaseClient,
    config: ConfigController,
    playlist: PlaylistController,
    players: PlayerController,
    prefs: PreferenceController,
    queue: QueueController,
    schedule: ScheduleController,
    ranking: ServerRankController,
    records: RecordController,
    race: RaceController,
    widget: WidgetController,
}

impl Controller {
    pub async fn init(config: Config, server: Server, db: DatabaseClient) -> Controller {
        // Lots and lots of dependency injection...

        // Controllers are up-casted to Live* traits, so that other controllers
        // can use cached data relevant for the current map/race/etc.
        // This facade will retain write access to update controller
        // states when receiving server events.

        // Using Arc<dyn T> everywhere to avoid lifetimes altogether.
        // We need 'static lifetimes, so that we can use controllers in Tokio tasks.
        // I *think* using something like Box<&'static dyn T> should be fine
        // as well, but I don't see any benefit.

        let config = ConfigController::init(&server, config).await;
        let live_config = Arc::new(config.clone()) as Arc<dyn LiveConfig>;

        let playlist = PlaylistController::init(&server, &db, &live_config).await;
        let live_playlist = Arc::new(playlist.clone()) as Arc<dyn LivePlaylist>;

        let players = PlayerController::init(&server, &db).await;
        let live_players = Arc::new(players.clone()) as Arc<dyn LivePlayers>;

        let prefs = PreferenceController::init(&server, &db, &live_playlist, &live_players).await;
        let live_prefs = Arc::new(prefs.clone()) as Arc<dyn LivePreferences>;

        let queue =
            QueueController::init(&server, &live_players, &live_playlist, &live_prefs).await;
        let live_queue = Arc::new(queue.clone()) as Arc<dyn LiveQueue>;

        let ranking = ServerRankController::init(&server, &db, &live_players).await;
        let live_server_ranking = Arc::new(ranking.clone()) as Arc<dyn LiveServerRanking>;

        let records = RecordController::init(&db, &live_playlist, &live_players).await;
        let live_records = Arc::new(records.clone()) as Arc<dyn LiveRecords>;

        let schedule = ScheduleController::init(
            &server,
            &db,
            &live_playlist,
            &live_queue,
            &live_records,
            &live_config,
        )
        .await;
        let live_schedule = Arc::new(schedule.clone()) as Arc<dyn LiveSchedule>;

        let race = RaceController::init(&server, &live_players).await;
        let live_race = Arc::new(race.clone()) as Arc<dyn LiveRace>;

        let widget = WidgetController::init(
            &server,
            &db,
            &live_config,
            &live_playlist,
            &live_players,
            &live_race,
            &live_records,
            &live_server_ranking,
            &live_prefs,
            &live_queue,
            &live_schedule,
        )
        .await;

        Controller {
            server,
            db,
            config,
            playlist,
            players,
            prefs,
            queue,
            schedule,
            ranking,
            records,
            race,
            widget,
        }
    }
}

/// Send a message to all players.
async fn announce(server: &Server, message: ServerMessage<'_>) {
    let message_str = message.to_string();
    if message_str.is_empty() {
        return;
    }
    log::debug!("server msg> {}", &message);
    server.chat_send(&message.to_string()).await;
}
