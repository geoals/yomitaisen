mod game;

pub use game::messages;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use game::{WordRepository, ephemeral::EphemeralState, matchmaking::MatchmakingState};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;

async fn health() -> &'static str {
    "ok"
}

#[derive(Clone)]
pub struct AppState {
    pub ephemeral: Arc<EphemeralState>,
    pub matchmaking: Arc<MatchmakingState>,
}

async fn ephemeral_ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_ephemeral_socket(socket, state))
}

async fn handle_ephemeral_socket(socket: WebSocket, state: AppState) {
    game::ephemeral::handle_connection(socket, state.ephemeral).await;
}

async fn matchmaking_ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_matchmaking_socket(socket, state))
}

async fn handle_matchmaking_socket(socket: WebSocket, state: AppState) {
    game::matchmaking::handle_connection(socket, state.matchmaking).await;
}

pub fn app(pool: SqlitePool) -> Router {
    app_with_config(pool, None)
}

pub fn app_with_config(pool: SqlitePool, round_timeout: Option<Duration>) -> Router {
    let word_repo = WordRepository::new(pool);

    let state = AppState {
        ephemeral: Arc::new(EphemeralState::new(word_repo.clone(), round_timeout)),
        matchmaking: Arc::new(MatchmakingState::new(word_repo, round_timeout)),
    };

    Router::new()
        .route("/health", get(health))
        .route("/ws/ephemeral", get(ephemeral_ws_handler))
        .route("/ws/matchmaking", get(matchmaking_ws_handler))
        .with_state(state)
}
