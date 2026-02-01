mod game;

pub use game::messages;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use game::{DuelState, WordRepository};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;

async fn health() -> &'static str {
    "ok"
}

#[derive(Clone)]
pub struct AppState {
    pub game: Arc<DuelState>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    game::handle_connection(socket, state.game).await;
}

pub fn app(pool: SqlitePool) -> Router {
    app_with_config(pool, None)
}

pub fn app_with_config(pool: SqlitePool, round_timeout: Option<Duration>) -> Router {
    let word_repo = WordRepository::new(pool);
    let mut duel_state = DuelState::new(word_repo);
    if let Some(timeout) = round_timeout {
        duel_state = duel_state.with_round_timeout(timeout);
    }
    let state = AppState {
        game: Arc::new(duel_state),
    };

    Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state)
}
