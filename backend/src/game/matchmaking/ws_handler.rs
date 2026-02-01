use super::state::{JoinResult, MatchmakingState};
use crate::game::duel::messages::{ClientMessage, ServerMessage};
use crate::game::duel::ws::{run_connection, ConnectionContext, ConnectionHandler};
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

impl ConnectionHandler for MatchmakingState {
    async fn handle_message(
        self: Arc<Self>,
        msg: ClientMessage,
        tx: broadcast::Sender<ServerMessage>,
        ctx: &mut ConnectionContext,
    ) {
        match msg {
            ClientMessage::Join { user_id } => {
                info!(user_id, "Player joining matchmaking");
                ctx.user_id = Some(user_id.clone());
                handle_join(&self, user_id, &tx).await;
            }
            ClientMessage::Answer { answer } => {
                let Some(user_id) = &ctx.user_id else {
                    warn!("Received answer from unknown user");
                    return;
                };
                self.registry.handle_answer(user_id, &answer, &tx).await;
            }
            ClientMessage::CreateGame { .. } | ClientMessage::JoinGame { .. } => {
                warn!("Received ephemeral game message on matchmaking endpoint");
                let _ = tx.send(ServerMessage::Error {
                    message: "Use /ws/ephemeral for create/join games".to_string(),
                });
            }
        }
    }

    fn handle_disconnect(&self, user_id: &str) {
        self.handle_disconnect(user_id);
    }

    fn name(&self) -> &'static str {
        "matchmaking"
    }
}

async fn handle_join(
    state: &MatchmakingState,
    user_id: String,
    tx: &broadcast::Sender<ServerMessage>,
) {
    match state.try_join(user_id.clone(), tx.clone()) {
        JoinResult::Waiting => {
            debug!(user_id, "Sending Waiting message");
            let _ = tx.send(ServerMessage::Waiting);
        }
        JoinResult::Matched {
            opponent_id,
            opponent_tx,
            game_id,
        } => {
            info!(game_id, user_id, opponent_id, "Game starting");

            let _ = tx.send(ServerMessage::GameStart {
                opponent: opponent_id.clone(),
            });
            let _ = opponent_tx.send(ServerMessage::GameStart {
                opponent: user_id,
            });

            // Start round 1
            state
                .registry
                .start_first_round(&game_id, &opponent_tx, tx)
                .await;
        }
    }
}

pub async fn handle_connection(socket: WebSocket, state: Arc<MatchmakingState>) {
    run_connection(socket, state).await;
}
