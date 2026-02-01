mod game;

pub use game::messages;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use game::{ephemeral::EphemeralState, matchmaking::MatchmakingState, WordRepository};
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

    let mut ephemeral_state = EphemeralState::new(word_repo.clone());
    let mut matchmaking_state = MatchmakingState::new(word_repo);

    if let Some(timeout) = round_timeout {
        ephemeral_state = ephemeral_state.with_round_timeout(timeout);
        matchmaking_state = matchmaking_state.with_round_timeout(timeout);
    }

    let state = AppState {
        ephemeral: Arc::new(ephemeral_state),
        matchmaking: Arc::new(matchmaking_state),
    };

    Router::new()
        .route("/health", get(health))
        .route("/ws/ephemeral", get(ephemeral_ws_handler))
        .route("/ws/matchmaking", get(matchmaking_ws_handler))
        .with_state(state)
}
