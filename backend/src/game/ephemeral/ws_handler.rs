use super::state::EphemeralState;
use crate::game::duel::messages::{ClientMessage, ServerMessage};
use crate::game::duel::ws::{run_connection, ConnectionContext, ConnectionHandler};
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::warn;

impl ConnectionHandler for EphemeralState {
    async fn handle_message(
        self: Arc<Self>,
        msg: ClientMessage,
        tx: broadcast::Sender<ServerMessage>,
        ctx: &mut ConnectionContext,
    ) {
        match msg {
            ClientMessage::CreateGame { player_name } => {
                ctx.user_id = Some(player_name.clone());
                let game_id = self.create_game(player_name, tx.clone());
                let _ = tx.send(ServerMessage::GameCreated { game_id });
                let _ = tx.send(ServerMessage::WaitingForOpponent);
            }
            ClientMessage::JoinGame {
                game_id,
                player_name,
            } => {
                let Some(joined) = self.join_game(&game_id, player_name, tx.clone()) else {
                    let _ = tx.send(ServerMessage::GameNotFound);
                    return;
                };
                // Set user_id to the (possibly modified) guest name
                ctx.user_id = Some(joined.guest_name.clone());

                // Notify host that opponent joined
                let _ = joined.host_tx.send(ServerMessage::OpponentJoined {
                    opponent_name: joined.guest_name.clone(),
                });

                // Send GameStart to both players
                let _ = joined.host_tx.send(ServerMessage::GameStart {
                    opponent: joined.guest_name.clone(),
                });
                let _ = tx.send(ServerMessage::GameStart {
                    opponent: joined.host_name.clone(),
                });

                // Start round 1
                self.registry
                    .start_first_round(&joined.game_id, &joined.host_tx, &tx)
                    .await;
            }
            ClientMessage::Answer { answer } => {
                let Some(user_id) = &ctx.user_id else {
                    warn!("Received answer from unknown user");
                    return;
                };
                self.registry.handle_answer(user_id, &answer, &tx).await;
            }
            ClientMessage::Join { .. } => {
                warn!("Received Join message on ephemeral endpoint");
                let _ = tx.send(ServerMessage::Error {
                    message: "Use /ws/matchmaking for authenticated matchmaking".to_string(),
                });
            }
        }
    }

    fn handle_disconnect(&self, user_id: &str) {
        self.handle_disconnect(user_id);
    }

    fn name(&self) -> &'static str {
        "ephemeral"
    }
}

pub async fn handle_connection(socket: WebSocket, state: Arc<EphemeralState>) {
    run_connection(socket, state).await;
}
