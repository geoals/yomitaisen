mod game;

pub use game::messages;

use axum::{
    Json, Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    http,
    response::Response,
    routing::get,
};
use game::{WordRepository, ephemeral::EphemeralState, ephemeral::LobbyList, matchmaking::MatchmakingState};
use tower_http::cors::{Any, CorsLayer};
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

const LOBBY_MAX_AGE_SECS: u64 = 300; // 5 minutes

async fn lobby_handler(State(state): State<AppState>) -> Json<LobbyList> {
    Json(state.ephemeral.list_pending_games(LOBBY_MAX_AGE_SECS))
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

    let cors_allow_all = std::env::var("CORS_ALLOW_ALL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let cors = if cors_allow_all {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::new()
            .allow_origin(["https://yomi.alsvik.cloud".parse().unwrap()])
            .allow_methods([http::Method::GET])
            .allow_headers([http::header::CONTENT_TYPE])
    };

    Router::new()
        .route("/health", get(health))
        .route("/lobby", get(lobby_handler))
        .route("/ws/ephemeral", get(ephemeral_ws_handler))
        .route("/ws/matchmaking", get(matchmaking_ws_handler))
        .layer(cors)
        .with_state(state)
}
