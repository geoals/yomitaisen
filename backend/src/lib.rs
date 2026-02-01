mod game;
mod messages;
mod repository;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use game::GameState;
use repository::WordRepository;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn health() -> &'static str {
    "ok"
}

#[derive(Clone)]
pub struct AppState {
    pub game: Arc<GameState>,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    game::handle_connection(socket, state.game).await;
}

pub fn app(pool: SqlitePool) -> Router {
    let word_repo = WordRepository::new(pool);
    let state = AppState {
        game: Arc::new(GameState::new(word_repo)),
    };

    Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state)
}
