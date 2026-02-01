use super::state::{JoinResult, MatchmakingState};
use crate::game::duel::active_game::{continue_or_end_game, spawn_round_timeout, AnswerResult};
use crate::game::duel::messages::{ClientMessage, ServerMessage};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

pub async fn handle_connection(socket: WebSocket, state: Arc<MatchmakingState>) {
    info!("New matchmaking WebSocket connection");
    let (mut sender, receiver) = socket.split();
    let (tx, mut rx) = broadcast::channel::<ServerMessage>(16);

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            debug!(?msg, "Sending message to client");
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let recv_task = tokio::spawn(handle_incoming(receiver, tx, state_clone));

    tokio::select! {
        _ = send_task => {},
        result = recv_task => {
            if let Ok(Some(user_id)) = result {
                state.handle_disconnect(&user_id);
            }
        },
    }

    info!("Matchmaking WebSocket connection closed");
}

struct ConnectionContext {
    user_id: Option<String>,
}

async fn handle_incoming(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    tx: broadcast::Sender<ServerMessage>,
    state: Arc<MatchmakingState>,
) -> Option<String> {
    let mut ctx = ConnectionContext { user_id: None };

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else {
            debug!("Received non-text message, ignoring");
            continue;
        };

        debug!(raw = %text, "Received message");

        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else {
            warn!(raw = %text, "Failed to parse client message");
            continue;
        };

        handle_message(client_msg, &tx, &state, &mut ctx).await;
    }

    ctx.user_id
}

async fn handle_message(
    msg: ClientMessage,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<MatchmakingState>,
    ctx: &mut ConnectionContext,
) {
    match msg {
        ClientMessage::Join { user_id } => {
            info!(user_id, "Player joining matchmaking");
            ctx.user_id = Some(user_id.clone());
            handle_join(user_id, tx, state).await;
        }
        ClientMessage::Answer { answer } => {
            let Some(user_id) = &ctx.user_id else {
                warn!("Received answer from unknown user");
                return;
            };
            debug!(user_id, answer, "Player answered");
            handle_answer(user_id, &answer, state, tx).await;
        }
        ClientMessage::CreateGame { .. } | ClientMessage::JoinGame { .. } => {
            // These are for ephemeral games, not matchmaking
            warn!("Received ephemeral game message on matchmaking endpoint");
            let _ = tx.send(ServerMessage::Error {
                message: "Use /ws/ephemeral for create/join games".to_string(),
            });
        }
    }
}

/// Create an Arc-wrapped cleanup function for the state
fn make_cleanup(state: Arc<MatchmakingState>) -> Arc<dyn crate::game::duel::CleanupGame> {
    Arc::new(move |game_id: &str| {
        state.cleanup_game(game_id);
    })
}

async fn handle_join(
    user_id: String,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<MatchmakingState>,
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
                opponent: user_id.clone(),
            });

            if let Some(word) = state.words.get_random().await {
                info!(
                    game_id,
                    kanji = word.kanji,
                    reading = word.reading,
                    "Round 1 starting"
                );

                let round_msg = ServerMessage::RoundStart {
                    kanji: word.kanji.clone(),
                    round: 1,
                };
                let _ = tx.send(round_msg.clone());
                let _ = opponent_tx.send(round_msg);

                if let Some(mut game) = state.games.get_mut(&game_id) {
                    game.session.start_round(1, word);
                }

                let cleanup = make_cleanup(state.clone());
                spawn_round_timeout(
                    state.round_timeout,
                    state.games.clone(),
                    state.words.clone(),
                    game_id,
                    1,
                    state.player_games.clone(),
                    cleanup,
                );
            }
        }
    }
}

async fn handle_answer(
    user_id: &str,
    answer: &str,
    state: &Arc<MatchmakingState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let Some(result) = submit_answer(state, user_id, answer) else {
        debug!(user_id, answer, "Wrong answer");
        let _ = tx.send(ServerMessage::WrongAnswer);
        return;
    };

    // Broadcast round result to both players
    state.broadcast_to_game(user_id, result.round_result);

    // Get game_id for continue_or_end_game
    let Some(game_id) = state.player_games.get(user_id).map(|r| r.clone()) else {
        return;
    };

    let cleanup = make_cleanup(state.clone());
    continue_or_end_game(
        &state.games,
        &state.words,
        state.round_timeout,
        &game_id,
        result.game_winner,
        result.round_number,
        &state.player_games,
        cleanup,
    )
    .await;
}

fn submit_answer(state: &MatchmakingState, user_id: &str, answer: &str) -> Option<AnswerResult> {
    let game_id = state.player_games.get(user_id)?;
    let mut game = state.games.get_mut(&*game_id)?;

    debug!(user_id, answer, "Player submitting answer");

    let outcome = game.session.submit_answer(user_id, answer)?;

    // Record the win
    if let Some(winner) = &outcome.winner {
        game.session.record_win(winner);
    }

    let scores = game.session.scores();
    let game_winner = game.session.game_winner().map(|s| s.to_string());

    info!(
        user_id,
        round_winner = ?outcome.winner,
        scores = ?scores,
        game_winner = ?game_winner,
        "Round ended"
    );

    let round_number = scores.0 + scores.1;

    Some(AnswerResult {
        round_result: ServerMessage::RoundResult {
            winner: outcome.winner,
            correct_reading: outcome.correct_reading,
        },
        game_winner,
        round_number,
    })
}
